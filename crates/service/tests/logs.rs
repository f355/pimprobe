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

use pimprobe_service::logs::LogStore;
use serde_json::{Value, json};

#[test]
fn completed_run_and_work_zero_survive_reopening() {
    let temp = tempfile::tempdir().unwrap();
    let logs = LogStore::open(temp.path().join("logs")).unwrap();
    logs.start(
        "run-1",
        "Outside X+ edge",
        "routine",
        json!({"family":"outside","x":1,"wcs":54}),
        json!({"position":[-100.0,-80.0,-20.0]}),
    )
    .unwrap();
    logs.trace("run-1", "contact", json!({"position":[-99.0,-80.0,-25.0]}))
        .unwrap();
    logs.finish(
        "run-1",
        "success",
        Some(json!({"point":[-98.0,null,null]})),
        None,
    )
    .unwrap();
    logs.action("run-1", "work_zero", json!({"offsets":[2,0,0]}))
        .unwrap();
    drop(logs);

    let reopened = LogStore::open(temp.path().join("logs")).unwrap();
    let entries = reopened.history().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["label"], "Outside X+ edge");
    assert_eq!(entries[0]["status"], "success");
    assert_eq!(entries[0]["result"]["point"][0], -98.0);
    assert_eq!(entries[0]["workZero"]["offsets"][0], 2);
    let trace = std::fs::read_to_string(temp.path().join("logs/diagnostics.jsonl")).unwrap();
    let events: Vec<Value> = trace
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        events
            .iter()
            .find(|event| event["event"] == "contact")
            .unwrap()["data"]["position"][0],
        -99.0
    );
}

#[test]
fn unfinished_run_appears_as_interrupted_after_restart() {
    let temp = tempfile::tempdir().unwrap();
    let logs = LogStore::open(temp.path()).unwrap();
    logs.start(
        "run-2",
        "Inside X/Y corner",
        "routine",
        json!({}),
        json!({}),
    )
    .unwrap();
    drop(logs);

    let reopened = LogStore::open(temp.path()).unwrap();
    let entries = reopened.history().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["status"], "interrupted");
}

#[test]
fn export_copies_history_and_diagnostics_then_clear_empties_both() {
    let temp = tempfile::tempdir().unwrap();
    let usb = tempfile::tempdir().unwrap();
    let logs = LogStore::open(temp.path().join("logs")).unwrap();
    logs.start("run-3", "Z surface", "routine", json!({}), json!({}))
        .unwrap();
    logs.finish("run-3", "failed", None, Some("No contact".into()))
        .unwrap();

    let exported = logs.export_to(usb.path()).unwrap();
    assert!(exported.join("history.jsonl").is_file());
    assert!(exported.join("diagnostics.jsonl").is_file());
    logs.clear().unwrap();
    assert!(logs.history().unwrap().is_empty());
    assert!(
        std::fs::read_to_string(temp.path().join("logs/diagnostics.jsonl"))
            .unwrap()
            .is_empty()
    );
    assert!(
        std::fs::read_to_string(exported.join("history.jsonl"))
            .unwrap()
            .contains("No contact")
    );
}

#[test]
fn diagnostic_trace_rotates_while_history_remains_available() {
    let temp = tempfile::tempdir().unwrap();
    let logs = LogStore::open(temp.path()).unwrap();
    logs.start("run-4", "Z surface", "routine", json!({}), json!({}))
        .unwrap();
    std::fs::write(
        temp.path().join("diagnostics.jsonl"),
        vec![b'x'; 10 * 1024 * 1024],
    )
    .unwrap();
    logs.trace("run-4", "progress", json!({"command":"G38.2 Z-1"}))
        .unwrap();
    assert!(temp.path().join("diagnostics.jsonl.1").exists());
    assert!(
        std::fs::read_to_string(temp.path().join("diagnostics.jsonl"))
            .unwrap()
            .contains("G38.2 Z-1")
    );
    assert_eq!(logs.history().unwrap()[0]["label"], "Z surface");
}
