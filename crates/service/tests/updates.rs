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

use axum::{
    Json, Router,
    body::{Body, Bytes},
    http::{HeaderValue, header},
    response::IntoResponse,
    routing::get,
};
use pimprobe_service::updates::{Release, ReleaseAsset, select_development, select_stable};
use pimprobe_service::updates::{UpdateError, UpdateManager};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{convert::Infallible, os::unix::fs::PermissionsExt};
use tokio_stream::StreamExt;

const TEST_COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn valid_installer() -> Vec<u8> {
    format!("#!/bin/sh\ncase \"$1\" in\n--check|--check-target) exit 0;;\n--extract) mkdir -p \"$2/app\"; printf '%s\\n' '{TEST_COMMIT}' >\"$2/app/COMMIT\"; printf '%s\\n' '2026.09.0' >\"$2/app/VERSION\";;\nesac\n").into_bytes()
}

fn release(tag: &str, prerelease: bool, commit: &str) -> Release {
    Release {
        tag: tag.into(),
        name: tag.into(),
        notes: format!("notes for {tag}"),
        prerelease,
        draft: false,
        commit: commit.into(),
        assets: vec![ReleaseAsset {
            name: if tag == "dev" {
                format!("pimprobe-linux-arm64-{}.run", &commit[..7])
            } else {
                format!("pimprobe-linux-arm64-{tag}.run")
            },
            url: format!("https://example.invalid/{tag}.run"),
            digest: Some(format!("sha256:{}", "01".repeat(32))),
        }],
    }
}

#[test]
fn stable_selection_uses_calendar_versions_and_allows_returning_from_dev() {
    let releases = vec![
        release("dev", true, "deadbeef"),
        release("2026.09.0", false, "a"),
        release("2026.09.2", false, "c"),
        release("2026.09.1", false, "b"),
        release("v1.0", false, "old"),
        release("2026.9.9", false, "bad-month"),
        release("2026.09.03", false, "bad-sequence"),
    ];
    assert_eq!(
        select_stable(&releases, "local-dirty").unwrap().tag,
        "2026.09.2"
    );
    assert_eq!(
        select_stable(&releases, "2026.09.1").unwrap().tag,
        "2026.09.2"
    );
    assert!(select_stable(&releases, "2026.09.2").is_none());
    assert!(select_stable(&releases, "2026.10.0").is_none());
}

#[test]
fn development_selection_only_compares_commit_identity() {
    let releases = vec![
        release("dev", true, "0123456789abcdef"),
        release("2026.09.0", false, "a"),
    ];
    assert!(select_development(&releases, "0123456").is_none());
    assert_eq!(
        select_development(&releases, "different").unwrap().tag,
        "dev"
    );
}

#[test]
fn releases_without_the_installer_or_digest_are_ignored() {
    let mut missing_digest = release("2026.09.1", false, "a");
    missing_digest.assets[0].digest = None;
    let mut wrong_asset = release("2026.09.2", false, "b");
    wrong_asset.assets[0].name = "source.zip".into();
    assert!(select_stable(&[missing_digest, wrong_asset], "local").is_none());
}

#[tokio::test]
async fn check_reads_installed_identity_and_release_metadata() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let digest = format!("sha256:{}", "01".repeat(32));
    let api = Router::new()
        .route("/releases", get(move || {
            let digest = digest.clone();
            async move { Json(json!([{
                "tag_name": "dev", "name": "Development Build", "body": "Fixes",
                "prerelease": true, "draft": false, "target_commitish": "main",
                "assets": [{"name": format!("pimprobe-linux-arm64-{}.run", &TEST_COMMIT[..7]), "browser_download_url": "http://example.invalid/update", "digest": digest}]
            }])) }
        }))
        .route("/commits/dev", get(|| async {
            Json(json!({"sha": TEST_COMMIT}))
        }));
    tokio::spawn(async move { axum::serve(listener, api).await.unwrap() });
    let temp = tempfile::tempdir().unwrap();
    let version = temp.path().join("VERSION");
    tokio::fs::write(&version, "1234567\n").await.unwrap();
    tokio::fs::write(temp.path().join("COMMIT"), format!("{TEST_COMMIT}\n"))
        .await
        .unwrap();
    let manager = pimprobe_service::updates::UpdateManager::new(
        format!("http://{address}/releases"),
        version,
        temp.path().into(),
    );
    let current = manager.check(true).await.unwrap();
    assert_eq!(current.current_version, "1234567");
    assert!(current.available.is_none());
    tokio::fs::write(temp.path().join("COMMIT"), format!("{}\n", "1".repeat(40)))
        .await
        .unwrap();
    let result = manager.check(true).await.unwrap();
    let available = result.available.unwrap();
    assert_eq!(available.version, "dev");
    assert_eq!(available.notes, "Fixes");
    assert!(available.development);
    assert!(!available.token.is_empty());
}

#[tokio::test]
async fn install_verifies_the_download_and_hands_it_to_a_transient_service() {
    let installer = valid_installer();
    let digest: String = Sha256::digest(&installer)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let releases_installer = installer.clone();
    let api = Router::new()
        .route("/releases", get(move || {
            let digest = digest.clone();
            async move { Json(json!([{
                "tag_name": "2026.09.0", "name": "First release", "body": "Notes",
                "prerelease": false, "draft": false, "target_commitish": "abcdef",
                "assets": [{"name": "pimprobe-linux-arm64-2026.09.0.run", "browser_download_url": format!("http://{address}/installer"), "digest": format!("sha256:{digest}")}]
            }])) }
        }))
        .route("/installer", get(move || {
            let bytes = releases_installer.clone();
            async move { bytes }
        }))
        .route("/commits/2026.09.0", get(|| async {
            Json(json!({"sha": TEST_COMMIT}))
        }));
    tokio::spawn(async move { axum::serve(listener, api).await.unwrap() });

    let temp = tempfile::tempdir().unwrap();
    let version = temp.path().join("VERSION");
    tokio::fs::write(&version, "local\n").await.unwrap();
    tokio::fs::write(
        temp.path().join("COMMIT"),
        "1111111111111111111111111111111111111111\n",
    )
    .await
    .unwrap();
    let record = temp.path().join("launch.txt");
    let launcher = temp.path().join("systemd-run");
    tokio::fs::write(
        &launcher,
        format!("#!/bin/sh\nprintf '%s\\n' \"$@\" >'{}'\nwhile [ \"$1\" != -- ]; do shift; done\nshift\nexec \"$@\" >/dev/null 2>&1\n", record.display()),
    )
    .await
    .unwrap();
    std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o755)).unwrap();
    let manager = pimprobe_service::updates::UpdateManager::new(
        format!("http://{address}/releases"),
        version,
        temp.path().into(),
    )
    .with_systemd_run(launcher);
    let token = manager.check(false).await.unwrap().available.unwrap().token;
    let newer_token = manager.check(false).await.unwrap().available.unwrap().token;
    assert_ne!(token, newer_token);
    let operation = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        manager.install(&token, || true),
    )
    .await
    .expect("the transient updater must exit promptly")
    .unwrap()
    .operation_id;
    let launch = tokio::fs::read_to_string(record).await.unwrap();
    assert!(launch.contains("--collect"), "{launch}");
    assert!(launch.contains("--yes --restart"), "{launch}");
    assert!(launch.contains("pimprobe-linux-arm64.run"), "{launch}");
    assert_eq!(manager.status(&operation).await.unwrap().state, "unknown");
    assert_eq!(update_stage_directories(temp.path()), 0);
    assert!(matches!(
        manager.install(&token, || true).await,
        Err(UpdateError::Expired)
    ));
    assert!(manager.install(&newer_token, || true).await.is_ok());
}

async fn failure_fixture(
    installer: Vec<u8>,
    advertised_digest: Option<String>,
    launcher_exit: i32,
    oversized: bool,
) -> (tempfile::TempDir, UpdateManager, String) {
    let actual: String = Sha256::digest(&installer)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let digest = advertised_digest.unwrap_or(actual);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let served = installer.clone();
    let api = Router::new()
        .route("/releases", get(move || {
            let digest = digest.clone();
            async move { Json(json!([{
                "tag_name": "2026.09.0", "name": "Release", "body": "Notes",
                "prerelease": false, "draft": false, "target_commitish": "abcdef",
                "assets": [{"name": "pimprobe-linux-arm64-2026.09.0.run", "browser_download_url": format!("http://{address}/installer"), "digest": format!("sha256:{digest}")}]
            }])) }
        }))
        .route("/installer", get(move || {
            let bytes = served.clone();
            async move {
                let mut response = if oversized {
                    let stream = tokio_stream::once(Ok::<_, Infallible>(Bytes::from(bytes)))
                        .chain(tokio_stream::pending());
                    Body::from_stream(stream).into_response()
                } else {
                    bytes.into_response()
                };
                if oversized {
                    response.headers_mut().insert(header::CONTENT_LENGTH, HeaderValue::from_static("134217729"));
                }
                response
            }
        }))
        .route("/commits/2026.09.0", get(|| async {
            Json(json!({"sha": TEST_COMMIT}))
        }));
    tokio::spawn(async move { axum::serve(listener, api).await.unwrap() });
    let temp = tempfile::tempdir().unwrap();
    let version = temp.path().join("VERSION");
    tokio::fs::write(&version, "local\n").await.unwrap();
    tokio::fs::write(
        temp.path().join("COMMIT"),
        "1111111111111111111111111111111111111111\n",
    )
    .await
    .unwrap();
    let launcher = temp.path().join("systemd-run");
    tokio::fs::write(&launcher, format!("#!/bin/sh\nexit {launcher_exit}\n"))
        .await
        .unwrap();
    std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o755)).unwrap();
    let manager = UpdateManager::new(
        format!("http://{address}/releases"),
        version,
        temp.path().into(),
    )
    .with_systemd_run(launcher);
    let token = manager.check(false).await.unwrap().available.unwrap().token;
    (temp, manager, token)
}

fn update_stage_directories(root: &std::path::Path) -> usize {
    std::fs::read_dir(root)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_ok_and(|kind| kind.is_dir())
                && entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("pimprobe-update.")
        })
        .count()
}

#[tokio::test]
async fn install_rejects_bad_digest_and_invalid_or_incompatible_installers() {
    let valid = valid_installer();
    let (temp, manager, token) = failure_fixture(valid, Some("00".repeat(32)), 0, false).await;
    assert!(matches!(
        manager.install(&token, || true).await,
        Err(UpdateError::Digest)
    ));
    assert_eq!(update_stage_directories(temp.path()), 0);

    let invalid = b"#!/bin/sh\nexit 1\n".to_vec();
    let (temp, manager, token) = failure_fixture(invalid, None, 0, false).await;
    assert!(matches!(
        manager.install(&token, || true).await,
        Err(UpdateError::InvalidInstaller)
    ));
    assert_eq!(update_stage_directories(temp.path()), 0);

    let wrong_identity = format!("#!/bin/sh\ncase \"$1\" in\n--check|--check-target) exit 0;;\n--extract) mkdir -p \"$2/app\"; echo {} >\"$2/app/COMMIT\";;\nesac\n", "b".repeat(40)).into_bytes();
    let (temp, manager, token) = failure_fixture(wrong_identity, None, 0, false).await;
    assert!(matches!(
        manager.install(&token, || true).await,
        Err(UpdateError::InvalidInstaller)
    ));
    assert_eq!(update_stage_directories(temp.path()), 0);

    let wrong_version = format!("#!/bin/sh\ncase \"$1\" in\n--check|--check-target) exit 0;;\n--extract) mkdir -p \"$2/app\"; echo {TEST_COMMIT} >\"$2/app/COMMIT\"; echo 2026.09.9 >\"$2/app/VERSION\";;\nesac\n").into_bytes();
    let (temp, manager, token) = failure_fixture(wrong_version, None, 0, false).await;
    assert!(matches!(
        manager.install(&token, || true).await,
        Err(UpdateError::InvalidInstaller)
    ));
    assert_eq!(update_stage_directories(temp.path()), 0);

    let incompatible = format!("#!/bin/sh\ncase \"$1\" in\n--check) exit 0;;\n--extract) mkdir -p \"$2/app\"; echo {TEST_COMMIT} >\"$2/app/COMMIT\"; echo 2026.09.0 >\"$2/app/VERSION\"; exit 0;;\n--check-target) echo unsupported >&2; exit 1;;\nesac\n").into_bytes();
    let (temp, manager, token) = failure_fixture(incompatible, None, 0, false).await;
    assert!(
        matches!(manager.install(&token, || true).await, Err(UpdateError::Target(message)) if message == "unsupported")
    );
    assert_eq!(update_stage_directories(temp.path()), 0);
}

#[tokio::test]
async fn failed_transient_launch_removes_staging_and_status_files() {
    let valid = valid_installer();
    let (temp, manager, token) = failure_fixture(valid, None, 1, false).await;
    assert!(matches!(
        manager.install(&token, || true).await,
        Err(UpdateError::Launch(_))
    ));
    assert_eq!(update_stage_directories(temp.path()), 0);
    assert_eq!(
        std::fs::read_dir(temp.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".status"))
            .count(),
        0
    );
}

#[tokio::test]
async fn install_rejects_an_oversized_download_before_reading_its_body() {
    let valid = b"#!/bin/sh\nexit 0\n".to_vec();
    let (temp, manager, token) = failure_fixture(valid, None, 0, true).await;
    let error = manager.install(&token, || true).await.unwrap_err();
    assert!(
        matches!(&error, UpdateError::Io(source) if source.to_string().contains("too large")),
        "{error:?}"
    );
    assert_eq!(update_stage_directories(temp.path()), 0);
}

#[tokio::test]
async fn install_rechecks_machine_state_before_handoff() {
    let valid = valid_installer();
    let (temp, manager, token) = failure_fixture(valid, None, 0, false).await;
    assert!(matches!(
        manager.install(&token, || false).await,
        Err(UpdateError::MachineBusy)
    ));
    assert_eq!(update_stage_directories(temp.path()), 0);
}
