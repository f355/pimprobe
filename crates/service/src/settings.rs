// PIMProbe - touch probing for the Nestworks C500.
// Copyright (c) 2026 Konstantin Tcepliaev <f355@f355.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use pimprobe_core::{NumericRange, Parameter};
use serde_json::{Map, Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("invalid setting: {0}")]
    Invalid(String),
    #[error("settings file: {0}")]
    Io(#[from] std::io::Error),
    #[error("settings JSON: {0}")]
    Json(#[from] serde_json::Error),
}

struct Rule {
    key: &'static str,
    default: Value,
    range: NumericRange,
}

fn rules() -> Vec<Rule> {
    let mut rules = Vec::new();
    for (key, value, parameter) in [
        ("centerXSearchDistance", 20., Parameter::SearchDistance),
        ("centerYSearchDistance", 20., Parameter::SearchDistance),
        ("centerDepth", 5., Parameter::Travel),
        ("probeDiameter", 2., Parameter::Diameter),
        ("retractDistance", 0.5, Parameter::Retract),
        ("safeZOffset", 40., Parameter::Travel),
        ("positioningFeed", 1000., Parameter::PositioningFeed),
        ("coarseFeed", 300., Parameter::ProbeFeed),
        ("fineFeed", 50., Parameter::ProbeFeed),
        ("outsideXSearchDistance", 10., Parameter::SearchDistance),
        ("outsideYSearchDistance", 10., Parameter::SearchDistance),
        ("outsideDepth", 5., Parameter::Travel),
        ("insideDepth", 5., Parameter::Travel),
        ("insideXSearchDistance", 10., Parameter::SearchDistance),
        ("insideYSearchDistance", 10., Parameter::SearchDistance),
    ] {
        rules.push(Rule {
            key,
            default: json!(value),
            range: parameter.range(),
        });
    }
    rules
}

pub fn schema() -> Map<String, Value> {
    rules()
        .into_iter()
        .map(|rule| {
            let mut entry = serde_json::to_value(rule.range).unwrap();
            entry["default"] = rule.default;
            (rule.key.to_owned(), entry)
        })
        .collect()
}

fn valid(rule: &Rule, value: &Value) -> bool {
    value.as_f64().is_some_and(|n| rule.range.contains(n))
}

pub struct Settings {
    path: PathBuf,
    values: Map<String, Value>,
}

impl Settings {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SettingsError> {
        let rules = rules();
        let mut values: Map<_, _> = rules
            .iter()
            .map(|r| (r.key.to_owned(), r.default.clone()))
            .collect();
        let path = path.as_ref().to_owned();
        let rewrite = match fs::read(&path) {
            Ok(data) => {
                let saved: Map<String, Value> = serde_json::from_slice(&data)?;
                for rule in &rules {
                    if let Some(value) = saved.get(rule.key).filter(|v| valid(rule, v)) {
                        values.insert(rule.key.to_owned(), value.clone());
                    }
                }
                saved != values
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
            Err(e) => return Err(e.into()),
        };
        let mut settings = Self { path, values };
        if rewrite {
            settings.update(Map::new())?;
        }
        Ok(settings)
    }

    pub fn snapshot(&self) -> Map<String, Value> {
        self.values.clone()
    }

    pub fn update(&mut self, patch: Map<String, Value>) -> Result<(), SettingsError> {
        let schema = rules();
        for (key, value) in &patch {
            if !schema.iter().any(|r| r.key == key && valid(r, value)) {
                return Err(SettingsError::Invalid(key.clone()));
            }
        }
        let mut values = self.values.clone();
        values.extend(patch);
        let parent = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer_pretty(&mut file, &values)?;
        file.write_all(b"\n")?;
        file.as_file().sync_all()?;
        file.persist(&self.path).map_err(|e| e.error)?;
        // Publish only after the replacement has reached disk.
        fs::File::open(parent)?.sync_all()?;
        self.values = values;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_and_loads_new_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut settings = Settings::open(&path).unwrap();
        assert_eq!(settings.snapshot()["coarseFeed"], 300.0);
        assert_eq!(settings.snapshot()["fineFeed"], 50.0);
        assert_eq!(settings.snapshot()["probeDiameter"], 2.0);
        assert_eq!(settings.snapshot()["safeZOffset"], 40.0);
        let routine = pimprobe_core::RoutineConfig::default();
        assert_eq!(routine.coarse_feed, 300.0);
        assert_eq!(routine.fine_feed, 50.0);
        assert_eq!(routine.diameter, 2.0);
        assert_eq!(routine.safe_z_offset, 40.0);
        settings
            .update(
                json!({"coarseFeed":123,"safeZOffset":25,"centerXSearchDistance":120,"centerYSearchDistance":150})
                    .as_object()
                    .unwrap()
                    .clone(),
            )
            .unwrap();
        let reopened = Settings::open(path).unwrap().snapshot();
        assert_eq!(reopened["coarseFeed"], 123);
        assert_eq!(reopened["centerXSearchDistance"], 120);
        assert_eq!(reopened["centerYSearchDistance"], 150);
        assert_eq!(reopened["safeZOffset"], 25);
        assert_eq!(reopened["outsideYSearchDistance"], 10.0);
    }

    #[test]
    fn rejects_entire_invalid_patch() {
        let dir = tempfile::tempdir().unwrap();
        let mut settings = Settings::open(dir.path().join("settings.json")).unwrap();
        let before = settings.snapshot();
        for patch in [
            json!({"coarseFeed":123,"retractDistance":0}),
            json!({"surprise":1}),
            json!({"safeZOffset":false}),
        ] {
            assert!(settings.update(patch.as_object().unwrap().clone()).is_err());
            assert_eq!(settings.snapshot(), before);
        }
    }

    #[test]
    fn loads_valid_values_defaults_invalid_values_and_rejects_malformed_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(
            &path,
            r#"{"oldField":1,"coarseFeed":-1,"centerXSearchDistance":40}"#,
        )
        .unwrap();
        let saved = Settings::open(&path).unwrap().snapshot();
        assert_eq!(saved["coarseFeed"], 300.0);
        assert_eq!(saved["centerXSearchDistance"], 40);
        let stored: Map<String, Value> = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(stored, saved);
        fs::write(&path, "{").unwrap();
        assert!(Settings::open(path).is_err());
    }
}
