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

import type { ProbeClient } from "../client";
import type { HistoryResult, Settings } from "../types";
import { Utilities } from "./Utilities";
import { History } from "./History";
import { Updates } from "./Updates";
import "./utilities.css";

export interface UtilityPageProps {
  page: "utilities" | "history" | "updates";
  client: ProbeClient;
  settings: Settings;
  onSettings: (patch: Partial<Settings>) => Promise<void>;
  onBack: () => void;
  onOpenResult: (result: HistoryResult) => void;
  onBusyChange?: (busy: boolean) => void;
}

export function UtilityPage(props: UtilityPageProps) {
  switch (props.page) {
    case "history":
      return <History {...props} />;
    case "updates":
      return <Updates {...props} />;
    default:
      return <Utilities {...props} />;
  }
}

export default UtilityPage;
