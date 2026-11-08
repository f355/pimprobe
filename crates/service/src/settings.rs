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

use pimprobe_app::settings::{JsonSettingsStore, ProbeSettings};
pub use pimprobe_app::settings::{SettingsError, schema};
use serde_json::{Map, Value};
use std::{fs, path::Path, sync::Arc};

pub fn open(path: impl AsRef<Path>) -> Result<pimprobe_app::settings::Settings, SettingsError> {
    open_with_version(
        path.as_ref(),
        Path::new("/userdata/cnc/apps/pimprobe/VERSION"),
    )
}

fn open_with_version(
    path: &Path,
    version_path: &Path,
) -> Result<pimprobe_app::settings::Settings, SettingsError> {
    let mut defaults = Map::new();
    if let Ok(version) = fs::read_to_string(version_path) {
        defaults.insert(
            "developmentUpdates".into(),
            Value::Bool(crate::updates::stable_version(version.trim()).is_none()),
        );
    }
    pimprobe_app::settings::Settings::load(Arc::new(JsonSettingsStore::new(
        path,
        ProbeSettings::from_saved(defaults),
    )))
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
            let mut settings = open_with_version(&path, &dir.path().join("VERSION")).unwrap();
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
        let stored: ProbeSettings = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(stored.snapshot(), saved);
        fs::write(&path, "{").unwrap();
        assert!(open(path).is_err());
    }
}
