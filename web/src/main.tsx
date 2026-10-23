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

import { createRoot } from "react-dom/client";
import { ProbeApp } from "./ProbeApp";
const parameters = new URLSearchParams(location.search);
if (parameters.has("preview")) {
  document.body.style.cssText = "display:grid;place-items:center;overflow:auto";
  document.getElementById("root")!.style.cssText =
    "width:800px;height:480px;flex:none";
}
createRoot(document.getElementById("root")!).render(
  <ProbeApp
    defaultLanguage={parameters.get("language") || ""}
    onLeave={() => window.close()}
  />,
);
