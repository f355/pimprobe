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

use pimprobe_core::{NumericRange, Parameter, RepeatabilitySettings};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone, Copy)]
struct FeedLimits {
    linear: f64,
    rotary: f64,
}

impl FeedLimits {
    fn from_settings(settings: &BTreeMap<i32, f64>) -> Option<Self> {
        let rate = |key| {
            settings
                .get(&key)
                .copied()
                .filter(|v| v.is_finite() && *v >= 1.)
        };
        Some(Self {
            linear: rate(110)?.min(rate(111)?).min(rate(112)?),
            rotary: rate(113)?,
        })
    }

    fn maximum(self, key: &str) -> Option<f64> {
        match key {
            "positioningFeed" | "coarseFeed" | "fineFeed" => Some(self.linear),
            "rotaryFeed" => Some(self.rotary),
            _ => None,
        }
    }
}

impl super::ProbeApp {
    async fn feed_limits(&self) -> Result<FeedLimits, super::AppError> {
        let mut settings = self.device.snapshot().settings;
        if let Some(limits) = FeedLimits::from_settings(&settings) {
            return Ok(limits);
        }
        let mut events = self.device.subscribe();
        self.device.send("$$").await?;
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                let event = events
                    .recv()
                    .await
                    .map_err(|_| pimprobe_core::Error::Disconnected)?;
                if let Some(code) = event.controller_error {
                    return Err(pimprobe_core::Error::Controller(format!(
                        "The machine reported error {code}."
                    )));
                }
                if let Some((key, value)) = event.setting {
                    settings.insert(key, value);
                    if let Some(limits) = FeedLimits::from_settings(&settings) {
                        return Ok(limits);
                    }
                }
            }
        })
        .await
        .map_err(|_| {
            super::AppError::conflict(
                "settings",
                "Could not read machine feed limits ($110-$113); reconnect and try again",
            )
        })?
        .map_err(Into::into)
    }

    pub async fn settings_schema(&self) -> Result<Map<String, Value>, super::AppError> {
        let _guard = self.acquire()?;
        let limits = self.feed_limits().await?;
        let mut schema = schema();
        for (key, entry) in &mut schema {
            if let Some(maximum) = limits.maximum(key) {
                entry["maximum"] = json!(maximum);
                entry["default"] = json!(entry["default"].as_f64().unwrap().min(maximum));
            }
        }
        Ok(schema)
    }

    pub(super) async fn validate_feeds(
        &self,
        values: &[(&str, f64)],
    ) -> Result<(), super::AppError> {
        let limits = self.feed_limits().await?;
        for &(key, value) in values {
            let maximum = limits.maximum(key).unwrap();
            if !(NumericRange {
                minimum: 1.,
                maximum,
            })
            .contains(value)
            {
                let label = match key {
                    "positioningFeed" => "Positioning feed",
                    "coarseFeed" => "Coarse feed",
                    "fineFeed" => "Fine feed",
                    "rotaryFeed" => "Rotary feed",
                    _ => unreachable!(),
                };
                return Err(super::AppError::invalid(
                    "settings",
                    format!(
                        "{label} must be between 1 and {maximum} {} (machine limit).",
                        if key == "rotaryFeed" {
                            "degrees/min"
                        } else {
                            "mm/min"
                        }
                    ),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("Invalid value for setting {0}.")]
    Invalid(String),
    #[error("Could not save settings: {0}")]
    Store(#[from] std::io::Error),
}

struct Rule {
    key: &'static str,
    default: Value,
    range: Option<NumericRange>,
}

fn rules() -> Vec<Rule> {
    let mut rules = Vec::new();
    for (key, value, parameter) in [
        ("centerXSearchDistance", 20., Parameter::SearchDistance),
        ("centerYSearchDistance", 20., Parameter::SearchDistance),
        ("centerDepth", 5., Parameter::Travel),
        ("probeDiameter", 2., Parameter::Diameter),
        ("retractDistance", 0.5, Parameter::Retract),
        ("safeZOffset", 40., Parameter::Travel),
        ("positioningFeed", 1000., Parameter::PositioningFeed),
        ("coarseFeed", 300., Parameter::ProbeFeed),
        ("fineFeed", 50., Parameter::ProbeFeed),
        ("outsideXSearchDistance", 10., Parameter::SearchDistance),
        ("outsideYSearchDistance", 10., Parameter::SearchDistance),
        ("outsideDepth", 5., Parameter::Travel),
        ("insideDepth", 5., Parameter::Travel),
        ("insideXSearchDistance", 10., Parameter::SearchDistance),
        ("insideYSearchDistance", 10., Parameter::SearchDistance),
    ] {
        rules.push(Rule {
            key,
            default: json!(value),
            range: Some(parameter.range()),
        });
    }
    for (key, value, minimum, maximum) in [
        ("rotaryRodDiameter", 10., 3., 100.),
        ("rotaryXDistance", 30., -200., 200.),
        ("rotaryFeed", 5000., 1., f64::MAX),
        ("rotaryYDistance", 10., 0.1, 100.),
        ("rotaryZDistance", 10., 0.1, 100.),
    ] {
        rules.push(Rule {
            key,
            default: json!(value),
            range: Some(NumericRange { minimum, maximum }),
        });
    }
    for (key, value) in [
        ("developmentUpdates", false),
        ("automaticUpdateChecks", true),
    ] {
        rules.push(Rule {
            key,
            default: json!(value),
            range: None,
        });
    }
    rules
}

pub fn schema() -> Map<String, Value> {
    rules()
        .into_iter()
        .map(|rule| {
            let mut entry = rule
                .range
                .map_or_else(|| json!({}), |range| serde_json::to_value(range).unwrap());
            entry["default"] = rule.default;
            (rule.key.to_owned(), entry)
        })
        .collect()
}

fn valid(rule: &Rule, value: &Value) -> bool {
    match rule.range {
        Some(range) => value.as_f64().is_some_and(|n| range.contains(n)),
        None => value.is_boolean(),
    }
}

/// Validated probing preferences. Serialization uses the public setting names.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ProbeSettings(Map<String, Value>);

impl Default for ProbeSettings {
    fn default() -> Self {
        Self(
            rules()
                .into_iter()
                .map(|r| (r.key.to_owned(), r.default))
                .collect(),
        )
    }
}

impl<'de> Deserialize<'de> for ProbeSettings {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let saved = Map::<String, Value>::deserialize(deserializer)?;
        Ok(Self::from_saved(saved))
    }
}

impl ProbeSettings {
    pub fn from_saved(saved: Map<String, Value>) -> Self {
        let mut values = Self::default();
        for rule in rules() {
            if let Some(value) = saved.get(rule.key).filter(|v| valid(&rule, v)) {
                values.0.insert(rule.key.to_owned(), value.clone());
            }
        }
        values
    }

    pub fn snapshot(&self) -> Map<String, Value> {
        self.0.clone()
    }

    pub fn patched(&self, patch: Map<String, Value>) -> Result<Self, SettingsError> {
        let schema = rules();
        for (key, value) in &patch {
            if !schema.iter().any(|r| r.key == key && valid(r, value)) {
                return Err(SettingsError::Invalid(key.clone()));
            }
        }
        let mut values = self.clone();
        values.0.extend(patch);
        Ok(values)
    }

    pub fn repeatability(&self) -> RepeatabilitySettings {
        RepeatabilitySettings {
            diameter: self.0["probeDiameter"].as_f64().unwrap(),
            retract: self.0["retractDistance"].as_f64().unwrap(),
            positioning_feed: self.0["positioningFeed"].as_f64().unwrap(),
            coarse_feed: self.0["coarseFeed"].as_f64().unwrap(),
            fine_feed: self.0["fineFeed"].as_f64().unwrap(),
        }
    }
}

/// Implementations own serialization and must report failed writes.
pub trait SettingsStore: Send + Sync {
    fn load(&self) -> std::io::Result<ProbeSettings>;
    fn save(&self, values: &ProbeSettings) -> std::io::Result<()>;
}

pub struct Settings {
    store: Arc<dyn SettingsStore>,
    values: ProbeSettings,
}
impl Settings {
    pub fn load(store: Arc<dyn SettingsStore>) -> Result<Self, SettingsError> {
        let values = store.load()?;
        Ok(Self { store, values })
    }
    pub fn snapshot(&self) -> Map<String, Value> {
        self.values.snapshot()
    }
    pub fn values(&self) -> &ProbeSettings {
        &self.values
    }
    pub fn update(&mut self, patch: Map<String, Value>) -> Result<(), SettingsError> {
        let values = self.values.patched(patch)?;
        self.store.save(&values)?;
        self.values = values;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    };

    #[test]
    fn feed_limits_use_all_linear_axes_and_the_rotary_axis() {
        let mut settings = [(110, 3000.), (111, 1000.), (112, 2500.), (113, 600.)].into();
        let limits = FeedLimits::from_settings(&settings).unwrap();
        assert_eq!(limits.maximum("fineFeed"), Some(1000.));
        assert_eq!(limits.maximum("rotaryFeed"), Some(600.));
        settings.insert(112, 500.);
        assert_eq!(FeedLimits::from_settings(&settings).unwrap().linear, 500.);
        settings.remove(&113);
        assert!(FeedLimits::from_settings(&settings).is_none());
    }

    #[derive(Default)]
    struct Store {
        saved: Mutex<ProbeSettings>,
        fail: AtomicBool,
    }
    impl SettingsStore for Store {
        fn load(&self) -> std::io::Result<ProbeSettings> {
            Ok(self.saved.lock().unwrap().clone())
        }
        fn save(&self, values: &ProbeSettings) -> std::io::Result<()> {
            if self.fail.load(Ordering::Relaxed) {
                return Err(std::io::Error::other("disk full"));
            }
            *self.saved.lock().unwrap() = values.clone();
            Ok(())
        }
    }

    #[test]
    fn publishes_only_successfully_saved_settings() {
        let store = Arc::new(Store::default());
        let mut settings = Settings::load(store.clone()).unwrap();
        let before = settings.snapshot();
        let patch = json!({"coarseFeed":123}).as_object().unwrap().clone();
        store.fail.store(true, Ordering::Relaxed);
        assert!(settings.update(patch.clone()).is_err());
        assert_eq!(settings.snapshot(), before);
        store.fail.store(false, Ordering::Relaxed);
        settings.update(patch).unwrap();
        assert_eq!(settings.snapshot()["coarseFeed"], 123);
        assert_eq!(Settings::load(store).unwrap().snapshot()["coarseFeed"], 123);
    }
}
