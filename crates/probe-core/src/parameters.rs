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

use serde::Serialize;

#[derive(Clone, Copy, Serialize)]
pub struct NumericRange {
    pub minimum: f64,
    pub maximum: f64,
}

impl NumericRange {
    pub fn contains(self, value: f64) -> bool {
        value.is_finite() && (self.minimum..=self.maximum).contains(&value)
    }
}

#[derive(Clone, Copy)]
pub enum Parameter {
    Travel,
    SearchDistance,
    Retract,
    Diameter,
    PositioningFeed,
    ProbeFeed,
}

impl Parameter {
    pub fn range(self) -> NumericRange {
        let (minimum, maximum) = match self {
            Self::Travel => (0.1, 100.0),
            Self::SearchDistance => (0.1, 1000.0),
            Self::Retract | Self::Diameter => (0.1, 20.0),
            Self::PositioningFeed => (1.0, 10000.0),
            Self::ProbeFeed => (1.0, 1000.0),
        };
        NumericRange { minimum, maximum }
    }
}
