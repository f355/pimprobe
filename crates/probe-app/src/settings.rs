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

use pimprobe_core::{NumericRange, Parameter, RepeatabilitySettings};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("invalid setting: {0}")]
    Invalid(String),
    #[error("settings store: {0}")]
    Store(#[from] std::io::Error),
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
    for (key, value, minimum, maximum) in [
        ("rotaryRodDiameter", 10., 3., 100.),
        ("rotaryXDistance", 30., -200., 200.),
        ("rotaryFeed", 360., 1., 3600.),
        ("rotaryYDistance", 10., 0.1, 100.),
        ("rotaryZDistance", 10., 0.1, 100.),
    ] {
        rules.push(Rule {
            key,
            default: json!(value),
            range: NumericRange { minimum, maximum },
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

/// Validated probing preferences. Serialization uses the public setting names.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ProbeSettings(Map<String, Value>);

impl Default for ProbeSettings {
    fn default() -> Self {
        Self(
            rules()
                .into_iter()
                .map(|r| (r.key.to_owned(), r.default))
                .collect(),
        )
    }
}

impl<'de> Deserialize<'de> for ProbeSettings {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let saved = Map::<String, Value>::deserialize(deserializer)?;
        Ok(Self::from_saved(saved))
    }
}

impl ProbeSettings {
    pub fn from_saved(saved: Map<String, Value>) -> Self {
        let mut values = Self::default();
        for rule in rules() {
            if let Some(value) = saved.get(rule.key).filter(|v| valid(&rule, v)) {
                values.0.insert(rule.key.to_owned(), value.clone());
            }
        }
        values
    }

    pub fn snapshot(&self) -> Map<String, Value> {
        self.0.clone()
    }

    pub fn patched(&self, patch: Map<String, Value>) -> Result<Self, SettingsError> {
        let schema = rules();
        for (key, value) in &patch {
            if !schema.iter().any(|r| r.key == key && valid(r, value)) {
                return Err(SettingsError::Invalid(key.clone()));
            }
        }
        let mut values = self.clone();
        values.0.extend(patch);
        Ok(values)
    }

    pub fn repeatability(&self) -> RepeatabilitySettings {
        RepeatabilitySettings {
            diameter: self.0["probeDiameter"].as_f64().unwrap(),
            retract: self.0["retractDistance"].as_f64().unwrap(),
            positioning_feed: self.0["positioningFeed"].as_f64().unwrap(),
            coarse_feed: self.0["coarseFeed"].as_f64().unwrap(),
            fine_feed: self.0["fineFeed"].as_f64().unwrap(),
        }
    }
}

/// Implementations own serialization and must report failed writes.
pub trait SettingsStore: Send + Sync {
    fn load(&self) -> std::io::Result<ProbeSettings>;
    fn save(&self, values: &ProbeSettings) -> std::io::Result<()>;
}

pub struct Settings {
    store: Arc<dyn SettingsStore>,
    values: ProbeSettings,
}
impl Settings {
    pub fn load(store: Arc<dyn SettingsStore>) -> Result<Self, SettingsError> {
        let values = store.load()?;
        Ok(Self { store, values })
    }
    pub fn snapshot(&self) -> Map<String, Value> {
        self.values.snapshot()
    }
    pub fn values(&self) -> &ProbeSettings {
        &self.values
    }
    pub fn update(&mut self, patch: Map<String, Value>) -> Result<(), SettingsError> {
        let values = self.values.patched(patch)?;
        self.store.save(&values)?;
        self.values = values;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    };

    #[derive(Default)]
    struct Store {
        saved: Mutex<ProbeSettings>,
        fail: AtomicBool,
    }
    impl SettingsStore for Store {
        fn load(&self) -> std::io::Result<ProbeSettings> {
            Ok(self.saved.lock().unwrap().clone())
        }
        fn save(&self, values: &ProbeSettings) -> std::io::Result<()> {
            if self.fail.load(Ordering::Relaxed) {
                return Err(std::io::Error::other("disk full"));
            }
            *self.saved.lock().unwrap() = values.clone();
            Ok(())
        }
    }

    #[test]
    fn publishes_only_successfully_saved_settings() {
        let store = Arc::new(Store::default());
        let mut settings = Settings::load(store.clone()).unwrap();
        let patch = json!({"coarseFeed":123}).as_object().unwrap().clone();
        store.fail.store(true, Ordering::Relaxed);
        assert!(settings.update(patch.clone()).is_err());
        assert_eq!(settings.snapshot()["coarseFeed"], 300.0);
        store.fail.store(false, Ordering::Relaxed);
        settings.update(patch).unwrap();
        assert_eq!(settings.snapshot()["coarseFeed"], 123);
        assert_eq!(Settings::load(store).unwrap().snapshot()["coarseFeed"], 123);
    }
}
