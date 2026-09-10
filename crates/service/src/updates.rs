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

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{fs, io::AsyncWriteExt, process::Command, sync::Mutex};
use uuid::Uuid;

const INSTALLER: &str = "pimprobe-linux-arm64.run";
const MAX_INSTALLER_SIZE: usize = 128 * 1024 * 1024;
const CANDIDATE_LIFETIME: Duration = Duration::from_secs(15 * 60);
const MAX_CANDIDATES: usize = 8;

#[derive(Clone, Debug, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    #[serde(rename = "browser_download_url")]
    pub url: String,
    pub digest: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Release {
    #[serde(rename = "tag_name")]
    pub tag: String,
    pub name: String,
    #[serde(rename = "body", default)]
    pub notes: String,
    pub prerelease: bool,
    pub draft: bool,
    #[serde(rename = "target_commitish")]
    pub commit: String,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableUpdate {
    pub token: String,
    pub version: String,
    pub name: String,
    pub notes: String,
    pub development: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    pub current_version: String,
    pub available: Option<AvailableUpdate>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallStarted {
    pub operation_id: String,
}

#[derive(Debug, Serialize)]
pub struct InstallStatus {
    pub state: String,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("Could not read the installed version: {0}")]
    Version(#[source] std::io::Error),
    #[error("Could not contact GitHub: {0}")]
    Network(#[from] reqwest::Error),
    #[error("GitHub returned invalid release metadata")]
    Metadata,
    #[error("The selected update is no longer available; check again")]
    Expired,
    #[error("The machine is no longer idle with the spindle stopped")]
    MachineBusy,
    #[error("The release installer has no SHA-256 digest")]
    MissingDigest,
    #[error("The downloaded installer failed its SHA-256 check")]
    Digest,
    #[error("Could not prepare the update: {0}")]
    Io(#[source] std::io::Error),
    #[error("The downloaded installer is not valid")]
    InvalidInstaller,
    #[error("The installer cannot run on this machine: {0}")]
    Target(String),
    #[error("Could not start the installer: {0}")]
    Launch(#[source] std::io::Error),
}

#[derive(Clone)]
struct Candidate {
    asset: ReleaseAsset,
    commit: String,
    version: String,
    created: Instant,
}

pub struct UpdateManager {
    client: reqwest::Client,
    releases_url: String,
    commits_url: String,
    version_file: PathBuf,
    commit_file: PathBuf,
    staging_root: PathBuf,
    status_root: PathBuf,
    systemd_run: PathBuf,
    candidates: Mutex<HashMap<String, Candidate>>,
}

impl UpdateManager {
    pub fn production() -> Self {
        let mut manager = Self::new(
            "https://api.github.com/repos/f355/pimprobe/releases".into(),
            "/userdata/pimprobe/VERSION".into(),
            "/userdata".into(),
        );
        manager.status_root = "/run".into();
        manager
    }

    pub fn new(releases_url: String, version_file: PathBuf, staging_root: PathBuf) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("PIMProbe updater")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("HTTP client configuration");
        let commits_url = format!("{}/commits", releases_url.trim_end_matches("/releases"));
        let commit_file = version_file.with_file_name("COMMIT");
        Self {
            client,
            releases_url,
            commits_url,
            version_file,
            commit_file,
            status_root: staging_root.clone(),
            staging_root,
            systemd_run: "systemd-run".into(),
            candidates: Mutex::new(HashMap::new()),
        }
    }

    pub fn with_systemd_run(mut self, command: PathBuf) -> Self {
        self.systemd_run = command;
        self
    }

    pub async fn check(&self, development: bool) -> Result<CheckResult, UpdateError> {
        let current = fs::read_to_string(&self.version_file)
            .await
            .map_err(UpdateError::Version)?
            .trim()
            .to_owned();
        let current_commit = fs::read_to_string(&self.commit_file)
            .await
            .map_err(UpdateError::Version)?
            .trim()
            .to_owned();
        let mut releases = self
            .client
            .get(&self.releases_url)
            .send()
            .await?
            .error_for_status()?
            .json::<Vec<Release>>()
            .await?;
        if development {
            let commit = self.resolve_commit("dev").await?;
            if let Some(release) = releases.iter_mut().find(|release| release.tag == "dev") {
                release.commit = commit;
            }
        }
        let selected = if development {
            select_development(&releases, &current_commit)
        } else {
            select_stable(&releases, &current)
        };
        let available = if let Some(release) = selected {
            let commit = if development {
                release.commit.clone()
            } else {
                self.resolve_commit(&release.tag).await?
            };
            let asset = installer_asset(&release)
                .expect("selected release has an installer")
                .clone();
            let version = if development {
                commit[..7].to_owned()
            } else {
                release.tag.clone()
            };
            let token = Uuid::new_v4().to_string();
            self.store_candidate(
                token.clone(),
                Candidate {
                    asset,
                    commit,
                    version,
                    created: Instant::now(),
                },
            )
            .await;
            Some(AvailableUpdate {
                token,
                version: release.tag.clone(),
                name: release.name.clone(),
                notes: release.notes.clone(),
                development,
            })
        } else {
            None
        };
        Ok(CheckResult {
            current_version: current,
            available,
        })
    }

    async fn store_candidate(&self, token: String, candidate: Candidate) {
        let now = Instant::now();
        let mut candidates = self.candidates.lock().await;
        candidates.retain(|_, value| now.duration_since(value.created) <= CANDIDATE_LIFETIME);
        while candidates.len() >= MAX_CANDIDATES {
            let oldest = candidates
                .iter()
                .min_by_key(|(_, value)| value.created)
                .map(|(key, _)| key.clone())
                .expect("non-empty candidate store");
            candidates.remove(&oldest);
        }
        candidates.insert(token, candidate);
    }

    async fn resolve_commit(&self, tag: &str) -> Result<String, UpdateError> {
        #[derive(Deserialize)]
        struct Commit {
            sha: String,
        }
        let sha = self
            .client
            .get(format!("{}/{tag}", self.commits_url))
            .send()
            .await?
            .error_for_status()?
            .json::<Commit>()
            .await?
            .sha;
        if sha.len() != 40 || !sha.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(UpdateError::Metadata);
        }
        Ok(sha)
    }

    pub async fn install<F>(&self, token: &str, ready: F) -> Result<InstallStarted, UpdateError>
    where
        F: FnOnce() -> bool,
    {
        let candidate = self
            .candidates
            .lock()
            .await
            .remove(token)
            .ok_or(UpdateError::Expired)?;
        if !candidate_is_fresh(candidate.created, Instant::now()) {
            return Err(UpdateError::Expired);
        }
        let expected = candidate
            .asset
            .digest
            .as_deref()
            .and_then(|v| v.strip_prefix("sha256:"))
            .filter(|v| v.len() == 64)
            .ok_or(UpdateError::MissingDigest)?;
        let mut response = self
            .client
            .get(&candidate.asset.url)
            .send()
            .await?
            .error_for_status()?;
        if response
            .content_length()
            .is_some_and(|size| size > MAX_INSTALLER_SIZE as u64)
        {
            return Err(UpdateError::Io(std::io::Error::other(
                "release installer is too large",
            )));
        }
        let stage_guard = tempfile::Builder::new()
            .prefix("pimprobe-update.")
            .tempdir_in(&self.staging_root)
            .map_err(UpdateError::Io)?;
        let stage = stage_guard.path().to_owned();
        let installer = stage.join(INSTALLER);
        let mut file = fs::File::create(&installer)
            .await
            .map_err(UpdateError::Io)?;
        let mut size = 0;
        let mut digest = Sha256::new();
        while let Some(chunk) = response.chunk().await? {
            size = checked_download_size(size, chunk.len())?;
            digest.update(&chunk);
            file.write_all(&chunk).await.map_err(UpdateError::Io)?;
        }
        file.flush().await.map_err(UpdateError::Io)?;
        drop(file);
        let actual = format!("{:x}", digest.finalize());
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(UpdateError::Digest);
        }
        let valid = Command::new("sh")
            .arg(&installer)
            .arg("--check")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(UpdateError::Io)?;
        if !valid.success() {
            return Err(UpdateError::InvalidInstaller);
        }
        let extracted = stage.join("metadata");
        let extraction = Command::new("sh")
            .arg(&installer)
            .arg("--extract")
            .arg(&extracted)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(UpdateError::Io)?;
        let embedded_identity = if extraction.success() {
            Some((
                fs::read_to_string(extracted.join("app/COMMIT")).await.ok(),
                fs::read_to_string(extracted.join("app/VERSION")).await.ok(),
            ))
        } else {
            None
        };
        let _ = fs::remove_dir_all(&extracted).await;
        if !embedded_identity.is_some_and(|(commit, version)| {
            commit.as_deref().map(str::trim) == Some(candidate.commit.as_str())
                && version.as_deref().map(str::trim) == Some(candidate.version.as_str())
        }) {
            return Err(UpdateError::InvalidInstaller);
        }
        let compatible = Command::new("sh")
            .arg(&installer)
            .arg("--check-target")
            .output()
            .await
            .map_err(UpdateError::Io)?;
        if !compatible.status.success() {
            let message = String::from_utf8_lossy(&compatible.stderr)
                .trim()
                .to_owned();
            return Err(UpdateError::Target(if message.is_empty() {
                "target validation failed".into()
            } else {
                message
            }));
        }
        if !ready() {
            return Err(UpdateError::MachineBusy);
        }
        let operation_id = Uuid::new_v4().to_string();
        let status_file = self.status_file(&operation_id)?;
        fs::write(&status_file, "starting\n")
            .await
            .map_err(UpdateError::Io)?;
        if let Err(error) = launch(&self.systemd_run, installer, stage.clone(), status_file).await {
            let _ = fs::remove_file(self.status_file(&operation_id)?).await;
            return Err(error);
        }
        let _stage = stage_guard.keep();
        Ok(InstallStarted { operation_id })
    }

    pub async fn status(&self, operation_id: &str) -> Result<InstallStatus, UpdateError> {
        let file = self.status_file(operation_id)?;
        let value = match fs::read_to_string(file).await {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => "unknown\n".into(),
            Err(error) => return Err(UpdateError::Io(error)),
        };
        let mut lines = value.lines();
        let status = InstallStatus {
            state: lines.next().unwrap_or("unknown").to_owned(),
            message: lines.collect::<Vec<_>>().join("\n"),
        };
        if status.state == "failed" {
            let _ = fs::remove_file(self.status_file(operation_id)?).await;
        }
        Ok(status)
    }

    fn status_file(&self, operation_id: &str) -> Result<PathBuf, UpdateError> {
        let id = Uuid::parse_str(operation_id).map_err(|_| UpdateError::Expired)?;
        Ok(self
            .status_root
            .join(format!("pimprobe-update-{id}.status")))
    }
}

fn candidate_is_fresh(created: Instant, now: Instant) -> bool {
    now.saturating_duration_since(created) <= CANDIDATE_LIFETIME
}

fn checked_download_size(current: usize, additional: usize) -> Result<usize, UpdateError> {
    current
        .checked_add(additional)
        .filter(|size| *size <= MAX_INSTALLER_SIZE)
        .ok_or_else(|| UpdateError::Io(std::io::Error::other("release installer is too large")))
}

fn installer_asset(release: &Release) -> Option<&ReleaseAsset> {
    let qualifier = if release.tag == "dev" {
        release.commit.get(..7)?
    } else {
        &release.tag
    };
    let expected = format!("pimprobe-linux-arm64-{qualifier}.run");
    release.assets.iter().find(|asset| {
        asset.name == expected
            && asset
                .digest
                .as_deref()
                .is_some_and(|v| v.starts_with("sha256:") && v.len() == 71)
    })
}

fn stable_version(tag: &str) -> Option<(u32, u32, u32)> {
    let mut parts = tag.split('.');
    let year = parts.next()?;
    let month = parts.next()?;
    let sequence = parts.next()?;
    if year.len() != 4
        || month.len() != 2
        || sequence.is_empty()
        || (sequence.len() > 1 && sequence.starts_with('0'))
        || parts.next().is_some()
    {
        return None;
    }
    let version = (
        year.parse().ok()?,
        month.parse().ok()?,
        sequence.parse().ok()?,
    );
    (1..=12).contains(&version.1).then_some(version)
}

pub fn select_stable(releases: &[Release], current: &str) -> Option<Release> {
    let latest = releases
        .iter()
        .filter(|r| !r.draft && !r.prerelease && installer_asset(r).is_some())
        .filter_map(|r| stable_version(&r.tag).map(|v| (v, r)))
        .max_by_key(|(v, _)| *v)?;
    if stable_version(current).is_some_and(|v| v >= latest.0) {
        None
    } else {
        Some(latest.1.clone())
    }
}

pub fn select_development(releases: &[Release], current: &str) -> Option<Release> {
    let release = releases
        .iter()
        .find(|r| r.tag == "dev" && r.prerelease && !r.draft && installer_asset(r).is_some())?;
    let same = current.len() >= 7
        && (release.commit.starts_with(current) || current.starts_with(&release.commit));
    (!same).then(|| release.clone())
}

async fn launch(
    command: &Path,
    installer: PathBuf,
    stage: PathBuf,
    status_file: PathBuf,
) -> Result<(), UpdateError> {
    let unit = format!("pimprobe-update-{}", Uuid::new_v4().simple());
    let script = "printf 'running\\n' >\"$3\"; if sh \"$1\" --yes --restart; then rm -f \"$3\"; else code=$?; printf 'failed\\nInstaller exited with status %s\\n' \"$code\" >\"$3\"; fi; rm -rf \"$2\"";
    let status = Command::new(command)
        .args(["--unit", &unit, "--collect", "--property=Type=exec", "--"])
        .arg("sh")
        .arg("-c")
        .arg(script)
        .arg("pimprobe-update")
        .arg(path_arg(&installer))
        .arg(path_arg(&stage))
        .arg(path_arg(&status_file))
        .status()
        .await
        .map_err(UpdateError::Launch)?;
    if status.success() {
        Ok(())
    } else {
        Err(UpdateError::Launch(std::io::Error::other(
            "systemd-run failed",
        )))
    }
}

fn path_arg(path: &Path) -> &std::ffi::OsStr {
    path.as_os_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_candidates_expire_before_installation() {
        let now = Instant::now();
        assert!(candidate_is_fresh(now, now));
        assert!(!candidate_is_fresh(
            now,
            now + CANDIDATE_LIFETIME + Duration::from_secs(1)
        ));
    }

    #[test]
    fn streamed_installer_size_is_bounded() {
        assert_eq!(
            checked_download_size(MAX_INSTALLER_SIZE - 1, 1).unwrap(),
            MAX_INSTALLER_SIZE
        );
        assert!(checked_download_size(MAX_INSTALLER_SIZE, 1).is_err());
    }

    #[tokio::test]
    async fn installation_failure_is_reported_once() {
        let temp = tempfile::tempdir().unwrap();
        let manager = UpdateManager::new(
            "https://example.invalid/releases".into(),
            temp.path().join("VERSION"),
            temp.path().into(),
        );
        let operation_id = Uuid::new_v4().to_string();
        fs::write(
            manager.status_file(&operation_id).unwrap(),
            "failed\nInstaller failed\n",
        )
        .await
        .unwrap();

        let failure = manager.status(&operation_id).await.unwrap();
        assert_eq!(failure.state, "failed");
        assert_eq!(failure.message, "Installer failed");
        assert_eq!(
            manager.status(&operation_id).await.unwrap().state,
            "unknown"
        );
    }
}
