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

import { useState } from "react";
import { createRoot } from "react-dom/client";
import {
  Button,
  ConfirmDialog,
  HeaderHelpContext,
  PageHeader,
} from "../controls";
import { I18nProvider, useTranslation } from "../i18n";
import { HelpProvider, helpProps } from "./context";
import { HelpGuide } from "./Guide";
import { actionHelp, optionHelp } from "./text";
import "../style.css";

function Fixture() {
  const { t, language } = useTranslation();
  const [active, setActive] = useState(false),
    [guide, setGuide] = useState(false);
  const [modal, setModal] = useState(false),
    [actions, setActions] = useState(0);
  const act = () => setActions((value) => value + 1);
  const header = {
    active,
    toggle: () => setActive((value) => !value),
    guide: () => setGuide(true),
  };
  return (
    <div className="pp-app">
      <HeaderHelpContext.Provider value={header}>
        <HelpProvider active={active && !guide} onActiveChange={setActive}>
          {guide ? (
            <HelpGuide onBack={() => setGuide(false)} />
          ) : (
            <>
              <PageHeader title={t("Probe settings")} onBack={act} />
              <div className="pp-content">
                <div className="pp-actions">
                  <Button onClick={act} help={actionHelp("Proceed")}>
                    {t("Proceed")}
                  </Button>
                  <Button
                    disabled
                    helpTitle="Probe settings"
                    help={optionHelp("probeDiameter")}
                  >
                    {t("Probe settings")}
                  </Button>
                  <Button onClick={() => setModal(true)}>Open dialog</Button>
                </div>
                <output data-testid="actions">{actions}</output>
                <output data-testid="interpolation">
                  {t("Set %1 zero", ["X"])} | {t("Unknown %1/%2", [0, "Y"])} |{" "}
                  {language}
                </output>
                <output hidden data-testid="message">
                  {t(
                    "Failed (error:5); history could not be saved: Permission denied",
                  )}
                </output>
                <div {...helpProps("Coordinates", "Read the coordinates.")}>
                  <button
                    type="button"
                    onPointerDown={act}
                    onClick={act}
                    data-testid="pointer-action"
                  >
                    Pointer action
                  </button>
                </div>
                <div
                  className="pp-scroll"
                  data-testid="scroll"
                  style={{ height: 160, flex: "none" }}
                >
                  {Array.from({ length: 12 }, (_, index) => (
                    <Button
                      key={index}
                      style={{
                        display: "flex",
                        width: "100%",
                        marginBottom: 8,
                      }}
                      helpTitle={`Row ${index}`}
                      help={optionHelp("probeDiameter")}
                      onClick={act}
                    >
                      Row {index}
                    </Button>
                  ))}
                </div>
              </div>
              {modal && (
                <ConfirmDialog
                  title="Modal"
                  message="Modal help targets"
                  onCancel={() => setModal(false)}
                  onConfirm={act}
                >
                  <Button onClick={header.toggle}>Toggle help</Button>
                  <Button
                    disabled
                    helpTitle="Modal disabled"
                    help={optionHelp("probeDiameter")}
                  >
                    Modal disabled
                  </Button>
                  <Button
                    onPointerDown={act}
                    onClick={act}
                    helpTitle="Modal action"
                    help={actionHelp("Proceed")}
                  >
                    Modal action
                  </Button>
                </ConfirmDialog>
              )}
            </>
          )}
        </HelpProvider>
      </HeaderHelpContext.Provider>
    </div>
  );
}

createRoot(document.getElementById("root")!).render(
  <I18nProvider
    language={new URLSearchParams(location.search).get("language") || "en_US"}
  >
    <Fixture />
  </I18nProvider>,
);
