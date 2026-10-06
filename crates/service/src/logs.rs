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

use serde_json::Value;
use std::{
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

    fn append_history(&self, event: &Value) -> io::Result<()> {
        append(&self.root.join("history.jsonl"), event, true)
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
        append(&path, event, false)
    }
}

fn append(path: &Path, event: &Value, durable: bool) -> io::Result<()> {
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
    let mut line = serde_json::to_vec(event).map_err(io::Error::other)?;
    line.push(b'\n');
    file.write_all(&line)?;
    if durable {
        file.sync_data()?;
    }
    Ok(())
}

fn timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

impl pimprobe_app::history::HistoryStore for LogStore {
    fn append(&self, record: &pimprobe_app::history::HistoryRecord) -> io::Result<()> {
        let _guard = self.lock.lock().unwrap();
        self.append_history(&serde_json::to_value(record).map_err(io::Error::other)?)
    }
    fn read(&self) -> io::Result<Vec<pimprobe_app::history::HistoryRecord>> {
        let _guard = self.lock.lock().unwrap();
        let file = File::open(self.root.join("history.jsonl"))?;
        let mut records = Vec::new();
        for line in BufReader::new(file).lines() {
            if let Ok(record) = serde_json::from_str(&line?) {
                records.push(record);
            }
        }
        Ok(records)
    }
    fn clear(&self) -> io::Result<()> {
        let _guard = self.lock.lock().unwrap();
        File::create(self.root.join("history.jsonl"))?;
        Ok(())
    }
}
impl pimprobe_app::history::Diagnostics for LogStore {
    fn record(&self, event: &pimprobe_app::history::DiagnosticEvent) -> io::Result<()> {
        let _guard = self.lock.lock().unwrap();
        self.append_trace(&serde_json::to_value(event).map_err(io::Error::other)?)
    }
    fn clear(&self) -> io::Result<()> {
        let _guard = self.lock.lock().unwrap();
        File::create(self.root.join("diagnostics.jsonl"))?;
        for index in 1..TRACE_FILES {
            let path = self.root.join(format!("diagnostics.jsonl.{index}"));
            if path.exists() {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }
}
