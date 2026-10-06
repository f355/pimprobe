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

use crate::logs::LogStore;
use pimprobe_app::{
    AppError,
    host::{ExportResult, HostActions, SoftwareInfo},
};
use pimprobe_core::async_trait;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct LocalHost {
    logs: Arc<LogStore>,
}
impl LocalHost {
    pub fn new(logs: Arc<LogStore>) -> Self {
        Self { logs }
    }
}

#[async_trait]
impl HostActions for LocalHost {
    fn software_info(&self) -> SoftwareInfo {
        let root = Path::new("/userdata/pimprobe");
        SoftwareInfo {
            version: fs::read_to_string(root.join("VERSION"))
                .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").into())
                .trim()
                .into(),
            commit: fs::read_to_string(root.join("COMMIT"))
                .unwrap_or_default()
                .trim()
                .into(),
        }
    }
    async fn export_logs(&self) -> Result<ExportResult, AppError> {
        let logs = self.logs.clone();
        tokio::task::spawn_blocking(move || {
            let mounts =
                fs::read_to_string("/proc/mounts").map_err(|e| AppError::storage("log", e))?;
            let destination = usb_mount(&mounts).ok_or_else(|| {
                AppError::conflict("usb", "Insert a mounted USB drive before exporting")
            })?;
            export_to(&logs, &destination)
        })
        .await
        .map_err(|e| AppError::storage("log", e))?
    }
}
pub fn export_to(logs: &LogStore, destination: &Path) -> Result<ExportResult, AppError> {
    let folder = logs
        .export_to(destination)
        .map_err(|e| AppError::storage("log", e))?;
    let relative = folder
        .strip_prefix(destination)
        .map_err(|e| AppError::storage("log", e))?;
    Ok(ExportResult {
        path: folder.display().to_string(),
        relative_path: relative.display().to_string(),
    })
}

fn usb_mount(mounts: &str) -> Option<PathBuf> {
    let mut candidates = mounts.lines().filter_map(|line| {
        let mut fields = line.split_whitespace();
        let source = fields.next()?;
        let target = fields.next()?.replace("\\040", " ");
        (source.starts_with("/dev/")
            && (target == "/mnt/udisk" || target.starts_with("/run/media/")))
        .then_some(PathBuf::from(target))
    });
    candidates
        .clone()
        .find(|path| path == Path::new("/mnt/udisk"))
        .or_else(|| candidates.next())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usb_mount_prefers_the_machine_mount_and_requires_a_block_device() {
        let mounts = "tmpfs /mnt/udisk tmpfs rw 0 0\n/dev/sdb1 /run/media/backup vfat rw 0 0\n/dev/sda1 /mnt/udisk vfat rw 0 0\n";
        assert_eq!(usb_mount(mounts), Some(PathBuf::from("/mnt/udisk")));
        assert_eq!(
            usb_mount("tmpfs /mnt/udisk tmpfs rw 0 0\n/dev/sdb1 /run/media/backup vfat rw 0 0\n"),
            Some(PathBuf::from("/run/media/backup"))
        );
        assert_eq!(usb_mount("tmpfs /mnt/udisk tmpfs rw 0 0\n"), None);
    }
}
