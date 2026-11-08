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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use super::{ProbeSettings, SettingsStore};
use serde_json::{Map, Value};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

const FILE_VERSION: u64 = 1;

pub struct JsonSettingsStore {
    path: PathBuf,
    defaults: ProbeSettings,
}

impl JsonSettingsStore {
    pub fn new(path: impl AsRef<Path>, defaults: ProbeSettings) -> Self {
        Self {
            path: path.as_ref().into(),
            defaults,
        }
    }
}

impl SettingsStore for JsonSettingsStore {
    fn load(&self) -> io::Result<ProbeSettings> {
        let mut saved: Map<String, Value> = match fs::read(&self.path) {
            Ok(data) => serde_json::from_slice(&data).map_err(io::Error::other)?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Map::new(),
            Err(e) => return Err(e),
        };
        let version = saved.remove("version").and_then(|value| value.as_u64());
        if version.is_none() && saved.get("probeDiameter").and_then(Value::as_f64) == Some(2.) {
            saved.remove("probeDiameter");
        }
        let mut values = self.defaults.snapshot();
        values.extend(saved.clone());
        let values = ProbeSettings::from_saved(values);
        if version != Some(FILE_VERSION) || values.snapshot() != saved {
            self.save(&values)?;
        }
        Ok(values)
    }

    fn save(&self, values: &ProbeSettings) -> io::Result<()> {
        let parent = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        if let Some(ancestor) = parent.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::File::open(ancestor)?.sync_all()?;
        }
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        let mut saved = values.snapshot();
        saved.insert("version".into(), FILE_VERSION.into());
        serde_json::to_writer_pretty(&mut file, &saved).map_err(io::Error::other)?;
        file.write_all(b"\n")?;
        file.as_file().sync_all()?;
        file.persist(&self.path).map_err(|e| e.error)?;
        fs::File::open(parent)?.sync_all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use serde_json::json;
    use std::sync::Arc;

    #[test]
    fn replaces_the_old_ball_default_once_and_preserves_calibrated_diameters() {
        for (saved, expected) in [
            (2., 1.83),
            (1.829, 1.829),
            (1.9999, 1.9999),
            (2.0001, 2.0001),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("settings.json");
            fs::write(
                &path,
                json!({"probeDiameter":saved,"fineFeed":37}).to_string(),
            )
            .unwrap();
            let store = JsonSettingsStore::new(&path, ProbeSettings::default());
            let values = store.load().unwrap().snapshot();
            assert_eq!(values["probeDiameter"], expected);
            assert_eq!(values["fineFeed"], 37);
            assert_eq!(store.load().unwrap().snapshot(), values);
            let calibrated = store
                .load()
                .unwrap()
                .patched(json!({"probeDiameter":2.}).as_object().unwrap().clone())
                .unwrap();
            store.save(&calibrated).unwrap();
            assert_eq!(store.load().unwrap().snapshot()["probeDiameter"], 2.);
        }
        assert_eq!(ProbeSettings::default().snapshot()["probeDiameter"], 1.83);
    }

    #[test]
    fn readers_reload_shared_calibration_and_edits_keep_other_clients_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let open = || {
            Settings::load(Arc::new(JsonSettingsStore::new(
                &path,
                ProbeSettings::default(),
            )))
            .unwrap()
        };
        let mut first = open();
        let mut second = open();
        first
            .update(json!({"probeDiameter":1.829}).as_object().unwrap().clone())
            .unwrap();
        second.reload().unwrap();
        assert_eq!(second.snapshot()["probeDiameter"], 1.829);
        second
            .update(json!({"fineFeed":40}).as_object().unwrap().clone())
            .unwrap();
        first
            .update(json!({"anglePointSpacing":30}).as_object().unwrap().clone())
            .unwrap();
        assert_eq!(first.snapshot()["fineFeed"], 40);
        assert_eq!(open().snapshot()["probeDiameter"], 1.829);
        fs::write(&path, "{").unwrap();
        assert!(first.reload().is_err());
        assert!(
            second
                .update(json!({"fineFeed":50}).as_object().unwrap().clone())
                .is_err()
        );
        assert_eq!(fs::read_to_string(path).unwrap(), "{");
    }
}
