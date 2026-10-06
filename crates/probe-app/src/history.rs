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
use serde_json::{Value, json};
use std::{
    io,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    pub schema: u32,
    pub id: String,
    pub timestamp_ms: u128,
    #[serde(flatten)]
    pub event: HistoryEvent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum HistoryEvent {
    Start {
        label: String,
        category: String,
        config: Value,
    },
    Finish {
        status: String,
        result: Option<Value>,
        error: Option<String>,
    },
    Action {
        action: String,
        data: Value,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp_ms: u128,
    pub label: String,
    pub category: String,
    pub config: Value,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<HistoryRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_zero: Option<Value>,
}

/// Record order must be preserved, including later work-zero actions.
pub trait HistoryStore: Send + Sync {
    fn append(&self, record: &HistoryRecord) -> io::Result<()>;
    fn read(&self) -> io::Result<Vec<HistoryRecord>>;
    fn clear(&self) -> io::Result<()>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticEvent {
    pub schema: u32,
    pub id: String,
    pub timestamp_ms: u128,
    pub event: String,
    pub data: Value,
}

pub trait Diagnostics: Send + Sync {
    fn record(&self, event: &DiagnosticEvent) -> io::Result<()>;
    fn clear(&self) -> io::Result<()>;
}

/// Keeps history and diagnostics ordered for each application action.
pub struct Records {
    history: Arc<dyn HistoryStore>,
    diagnostics: Arc<dyn Diagnostics>,
}
impl Records {
    pub fn new(history: Arc<dyn HistoryStore>, diagnostics: Arc<dyn Diagnostics>) -> Self {
        Self {
            history,
            diagnostics,
        }
    }
    pub fn start(
        &self,
        id: &str,
        label: &str,
        category: &str,
        config: Value,
        state: Value,
    ) -> io::Result<()> {
        self.append(
            id,
            HistoryEvent::Start {
                label: label.into(),
                category: category.into(),
                config: config.clone(),
            },
        )?;
        self.trace(
            id,
            "start",
            json!({"label":label,"category":category,"config":config,"state":state}),
        )
    }
    pub fn finish(
        &self,
        id: &str,
        status: &str,
        result: Option<Value>,
        error: Option<String>,
    ) -> io::Result<()> {
        self.append(
            id,
            HistoryEvent::Finish {
                status: status.into(),
                result,
                error,
            },
        )
    }
    pub fn action(&self, id: &str, action: &str, data: Value) -> io::Result<()> {
        self.append(
            id,
            HistoryEvent::Action {
                action: action.into(),
                data,
            },
        )
    }
    fn append(&self, id: &str, event: HistoryEvent) -> io::Result<()> {
        let record = HistoryRecord {
            schema: 1,
            id: id.into(),
            timestamp_ms: timestamp_ms(),
            event,
        };
        self.history.append(&record)?;
        let value = serde_json::to_value(&record.event).map_err(io::Error::other)?;
        self.trace(
            id,
            value["event"].as_str().unwrap_or("history"),
            value.clone(),
        )
    }
    pub fn trace(&self, id: &str, event: &str, data: Value) -> io::Result<()> {
        self.diagnostics.record(&DiagnosticEvent {
            schema: 1,
            id: id.into(),
            timestamp_ms: timestamp_ms(),
            event: event.into(),
            data,
        })
    }
    pub fn history(&self) -> io::Result<Vec<HistoryEntry>> {
        let mut entries = Vec::<HistoryEntry>::new();
        let mut indices = std::collections::HashMap::<String, usize>::new();
        for record in self.history.read()? {
            match &record.event {
                HistoryEvent::Start {
                    label,
                    category,
                    config,
                } => {
                    indices.insert(record.id.clone(), entries.len());
                    entries.push(HistoryEntry {
                        id: record.id.clone(),
                        timestamp_ms: record.timestamp_ms,
                        label: label.clone(),
                        category: category.clone(),
                        config: config.clone(),
                        status: "interrupted".into(),
                        result: None,
                        error: None,
                        actions: Vec::new(),
                        work_zero: None,
                    });
                }
                HistoryEvent::Finish {
                    status,
                    result,
                    error,
                } => {
                    if let Some(&index) = indices.get(&record.id) {
                        entries[index].status = status.clone();
                        entries[index].result = result.clone();
                        entries[index].error = error.clone();
                    }
                }
                HistoryEvent::Action { action, data } => {
                    if let Some(&index) = indices.get(&record.id) {
                        if action == "work_zero" {
                            entries[index].work_zero = Some(data.clone());
                        }
                        entries[index].actions.push(record);
                    }
                }
            }
        }
        entries.reverse();
        entries.truncate(100);
        Ok(entries)
    }
    pub fn clear(&self) -> io::Result<()> {
        self.history.clear()?;
        self.diagnostics.clear()
    }
}
fn timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
