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

import { Button, Notice, PageHeader } from "./controls";
import { useTranslation } from "./i18n";
export function WcsPicker({
  current,
  onBack,
  onSelect,
  pending,
  title,
  error,
}: {
  current: number;
  onBack: () => void;
  onSelect: (wcs: number) => void;
  pending?: boolean;
  title?: string;
  error?: string;
}) {
  const { t } = useTranslation();
  return (
    <>
      <PageHeader
        title={title || t("Work coordinates")}
        onBack={onBack}
        backDisabled={pending}
      />
      <div className="pp-content">
        {error && <Notice error>{t(error)}</Notice>}
        <div className="pp-wcs-grid">
          {[54, 55, 56, 57, 58, 59].map((wcs) => (
            <Button
              key={wcs}
              className={current === wcs ? "pp-selected" : ""}
              aria-pressed={current === wcs}
              disabled={pending}
              onClick={() => onSelect(wcs)}
              help={t(
                "Select this work coordinate system for readouts and new measurements.",
              )}
            >
              G{wcs}
            </Button>
          ))}
        </div>
      </div>
    </>
  );
}
