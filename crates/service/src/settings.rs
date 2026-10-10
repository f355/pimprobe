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

use pimprobe_app::settings::{ProbeSettings, SettingsStore};
pub use pimprobe_app::settings::{SettingsError, schema};
use serde_json::{Map, Value};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct JsonSettingsStore {
    path: PathBuf,
}
impl JsonSettingsStore {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_owned(),
        }
    }
}
impl SettingsStore for JsonSettingsStore {
    fn load(&self) -> std::io::Result<ProbeSettings> {
        let mut saved: Map<String, Value> = match fs::read(&self.path) {
            Ok(data) => serde_json::from_slice(&data).map_err(std::io::Error::other)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Map::new(),
            Err(e) => return Err(e),
        };
        if !saved.contains_key("developmentUpdates")
            && let Ok(version) = fs::read_to_string(self.path.with_file_name("VERSION"))
        {
            saved.insert(
                "developmentUpdates".into(),
                Value::Bool(crate::updates::stable_version(version.trim()).is_none()),
            );
        }
        let values = ProbeSettings::from_saved(saved.clone());
        if values.snapshot() != saved {
            self.save(&values)?;
        }
        Ok(values)
    }
    fn save(&self, values: &ProbeSettings) -> std::io::Result<()> {
        let parent = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer_pretty(&mut file, values).map_err(std::io::Error::other)?;
        file.write_all(b"\n")?;
        file.as_file().sync_all()?;
        file.persist(&self.path).map_err(|e| e.error)?;
        fs::File::open(parent)?.sync_all()
    }
}

pub fn open(path: impl AsRef<Path>) -> Result<pimprobe_app::settings::Settings, SettingsError> {
    pimprobe_app::settings::Settings::load(Arc::new(JsonSettingsStore::new(path)))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn saves_edits_and_loads_missing_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut settings = open(&path).unwrap();
        let defaults = settings.snapshot();
        settings
            .update(
                json!({"coarseFeed":123,"safeZOffset":25,"centerXSearchDistance":120,"centerYSearchDistance":150,"developmentUpdates":true,"automaticUpdateChecks":false})
                    .as_object()
                    .unwrap()
                    .clone(),
            )
            .unwrap();
        let reopened = open(path).unwrap().snapshot();
        assert_eq!(reopened["coarseFeed"], 123);
        assert_eq!(reopened["centerXSearchDistance"], 120);
        assert_eq!(reopened["centerYSearchDistance"], 150);
        assert_eq!(reopened["safeZOffset"], 25);
        assert_eq!(reopened["developmentUpdates"], true);
        assert_eq!(reopened["automaticUpdateChecks"], false);
        assert_eq!(
            reopened["outsideYSearchDistance"],
            defaults["outsideYSearchDistance"]
        );
    }

    #[test]
    fn rejects_entire_invalid_patch() {
        let dir = tempfile::tempdir().unwrap();
        let mut settings = open(dir.path().join("settings.json")).unwrap();
        let before = settings.snapshot();
        for patch in [
            json!({"coarseFeed":123,"retractDistance":0}),
            json!({"surprise":1}),
            json!({"safeZOffset":false}),
            json!({"developmentUpdates":1}),
            json!({"automaticUpdateChecks":"yes"}),
        ] {
            assert!(settings.update(patch.as_object().unwrap().clone()).is_err());
            assert_eq!(settings.snapshot(), before);
        }
    }

    #[test]
    fn update_channel_starts_with_the_installed_build_and_keeps_the_saved_choice() {
        for (version, development) in [("2026.10.0", false), ("abcdef0", true)] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("settings.json");
            fs::write(dir.path().join("VERSION"), version).unwrap();
            let mut saved = ProbeSettings::default().snapshot();
            saved.remove("developmentUpdates");
            saved.remove("automaticUpdateChecks");
            fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
            let mut settings = open(&path).unwrap();
            assert_eq!(settings.snapshot()["developmentUpdates"], development);
            assert_eq!(settings.snapshot()["automaticUpdateChecks"], true);
            settings
                .update(
                    json!({"developmentUpdates": !development})
                        .as_object()
                        .unwrap()
                        .clone(),
                )
                .unwrap();
            assert_eq!(
                open(path).unwrap().snapshot()["developmentUpdates"],
                !development
            );
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
        let saved = open(&path).unwrap().snapshot();
        assert_eq!(
            saved["coarseFeed"],
            ProbeSettings::default().snapshot()["coarseFeed"]
        );
        assert_eq!(saved["centerXSearchDistance"], 40);
        let stored: Map<String, Value> = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(stored, saved);
        fs::write(&path, "{").unwrap();
        assert!(open(path).is_err());
    }
}
