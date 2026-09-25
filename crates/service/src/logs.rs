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

use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

const TRACE_BYTES: u64 = 10 * 1024 * 1024;
const TRACE_FILES: usize = 5;

pub struct LogStore {
    root: PathBuf,
    lock: Mutex<()>,
}

impl LogStore {
    pub fn open(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref().to_owned();
        fs::create_dir_all(&root)?;
        for name in ["history.jsonl", "diagnostics.jsonl"] {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(root.join(name))?;
        }
        Ok(Self {
            root,
            lock: Mutex::new(()),
        })
    }

    pub fn start(
        &self,
        id: &str,
        label: &str,
        category: &str,
        config: Value,
        state: Value,
    ) -> io::Result<()> {
        let _guard = self.lock.lock().unwrap();
        let timestamp = timestamp_ms();
        self.append_history(&json!({
            "schema":1, "event":"start", "id":id, "timestampMs":timestamp,
            "label":label, "category":category, "config":config
        }))?;
        self.append_trace(&json!({
            "schema":1, "event":"start", "id":id, "timestampMs":timestamp,
            "label":label, "category":category, "config":config, "state":state
        }))
    }

    pub fn trace(&self, id: &str, event: &str, data: Value) -> io::Result<()> {
        let _guard = self.lock.lock().unwrap();
        self.append_trace(&json!({
            "schema":1, "event":event, "id":id, "timestampMs":timestamp_ms(), "data":data
        }))
    }

    pub fn finish(
        &self,
        id: &str,
        status: &str,
        result: Option<Value>,
        error: Option<String>,
    ) -> io::Result<()> {
        let _guard = self.lock.lock().unwrap();
        let event = json!({
            "schema":1, "event":"finish", "id":id, "timestampMs":timestamp_ms(),
            "status":status, "result":result, "error":error
        });
        self.append_history(&event)?;
        self.append_trace(&event)
    }

    pub fn action(&self, id: &str, action: &str, data: Value) -> io::Result<()> {
        let _guard = self.lock.lock().unwrap();
        let event = json!({
            "schema":1, "event":"action", "id":id, "timestampMs":timestamp_ms(),
            "action":action, "data":data
        });
        self.append_history(&event)?;
        self.append_trace(&event)
    }

    pub fn history(&self) -> io::Result<Vec<Value>> {
        let _guard = self.lock.lock().unwrap();
        let file = File::open(self.root.join("history.jsonl"))?;
        let mut entries = Vec::<Value>::new();
        let mut indices = HashMap::<String, usize>::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            let Ok(event) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let Some(id) = event["id"].as_str() else {
                continue;
            };
            match event["event"].as_str() {
                Some("start") => {
                    indices.insert(id.to_owned(), entries.len());
                    entries.push(json!({
                        "id":id, "timestampMs":event["timestampMs"],
                        "label":event["label"], "category":event["category"],
                        "config":event["config"], "status":"interrupted"
                    }));
                }
                Some("finish") => {
                    if let Some(&index) = indices.get(id) {
                        entries[index]["status"] = event["status"].clone();
                        entries[index]["result"] = event["result"].clone();
                        entries[index]["error"] = event["error"].clone();
                    }
                }
                Some("action") => {
                    if let Some(&index) = indices.get(id) {
                        let actions = entries[index]["actions"].as_array_mut();
                        if let Some(actions) = actions {
                            actions.push(event.clone());
                        } else {
                            entries[index]["actions"] = json!([event]);
                        }
                        if event["action"] == "work_zero" {
                            entries[index]["workZero"] = event["data"].clone();
                        }
                    }
                }
                _ => {}
            }
        }
        entries.reverse();
        entries.truncate(100);
        Ok(entries)
    }

    pub fn export_to(&self, destination: &Path) -> io::Result<PathBuf> {
        let _guard = self.lock.lock().unwrap();
        let stamp = timestamp_ms() / 1000;
        let mut folder = destination.join(format!("PIMProbe-logs-{stamp}"));
        for suffix in 1.. {
            if !folder.exists() {
                break;
            }
            folder = destination.join(format!("PIMProbe-logs-{stamp}-{suffix}"));
        }
        fs::create_dir(&folder)?;
        for name in ["history.jsonl", "diagnostics.jsonl"] {
            fs::copy(self.root.join(name), folder.join(name))?;
        }
        for index in 1..TRACE_FILES {
            let name = format!("diagnostics.jsonl.{index}");
            if self.root.join(&name).exists() {
                fs::copy(self.root.join(&name), folder.join(name))?;
            }
        }
        Ok(folder)
    }

    pub fn clear(&self) -> io::Result<()> {
        let _guard = self.lock.lock().unwrap();
        for name in ["history.jsonl", "diagnostics.jsonl"] {
            File::create(self.root.join(name))?;
        }
        for index in 1..TRACE_FILES {
            let path = self.root.join(format!("diagnostics.jsonl.{index}"));
            if path.exists() {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }

    fn append_history(&self, event: &Value) -> io::Result<()> {
        append(&self.root.join("history.jsonl"), event)
    }

    fn append_trace(&self, event: &Value) -> io::Result<()> {
        let path = self.root.join("diagnostics.jsonl");
        let line = serde_json::to_vec(event).map_err(io::Error::other)?;
        if path.metadata()?.len() + line.len() as u64 + 1 > TRACE_BYTES {
            for index in (1..TRACE_FILES).rev() {
                let old = self.root.join(format!("diagnostics.jsonl.{index}"));
                if index == TRACE_FILES - 1 && old.exists() {
                    fs::remove_file(&old)?;
                }
                let previous = if index == 1 {
                    path.clone()
                } else {
                    self.root.join(format!("diagnostics.jsonl.{}", index - 1))
                };
                if previous.exists() {
                    fs::rename(previous, old)?;
                }
            }
        }
        append(&path, event)
    }
}

fn append(path: &Path, event: &Value) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .open(path)?;
    if file.metadata()?.len() > 0 {
        file.seek(SeekFrom::End(-1))?;
        let mut last = [0];
        file.read_exact(&mut last)?;
        if last[0] != b'\n' {
            file.write_all(b"\n")?;
        }
    }
    serde_json::to_writer(&mut file, event).map_err(io::Error::other)?;
    file.write_all(b"\n")
}

fn timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
