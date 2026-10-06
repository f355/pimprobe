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

use super::*;
use pimprobe_core::{RotaryConfig, RotaryResult};

pub(super) async fn review(
    State(app): State<Arc<App>>,
    ApiJson(config): ApiJson<RotaryConfig>,
) -> Result<Json<pimprobe_app::RotaryReview>, ApiError> {
    Ok(Json(app.probe.review_rotary(config).await?))
}
pub(super) async fn run(
    State(app): State<Arc<App>>,
    ApiJson(token): ApiJson<Token>,
) -> Result<Response, ApiError> {
    Ok(stream(app.probe.run_rotary(token).await?))
}
pub(super) async fn zero(
    State(app): State<Arc<App>>,
    ApiJson(token): ApiJson<Token>,
) -> Result<Json<RotaryResult>, ApiError> {
    Ok(Json(app.probe.apply_rotary(token, false).await?))
}
pub(super) async fn rotation(
    State(app): State<Arc<App>>,
    ApiJson(token): ApiJson<Token>,
) -> Result<Json<RotaryResult>, ApiError> {
    Ok(Json(app.probe.apply_rotary(token, true).await?))
}
