// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Konstantin Tcepliaev <f355@f355.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

import { useId } from "react";
import { useTranslation } from "../i18n";

export function BedReferenceDiagram() {
  const { t } = useTranslation();
  const hatch = useId();
  return (
    <svg
      className="pp-bed-diagram"
      viewBox="0 0 924 332"
      role="img"
      aria-label={t(
        "Top and side views of the probe ball in the counterbore, clear of the sides and screw",
      )}
      fill="#eef3f4"
      fontFamily="system-ui, sans-serif"
    >
      <text x="227" y="26" fontSize="22" fontWeight="650" textAnchor="middle">
        {t("Top view")}
      </text>
      <text x="697" y="26" fontSize="22" fontWeight="650" textAnchor="middle">
        {t("Side view")}
      </text>

      <svg
        x="0"
        y="38"
        width="454"
        height="286"
        viewBox="447 149 500 322"
        preserveAspectRatio="xMidYMid slice"
      >
        <rect x="447" y="149" width="500" height="322" fill="#202b30" />
        <path d="M518 149H947V471H518Z" fill="#5c6d73" />
        <path d="M522 149H947V471H522Z" fill="#75868c" />
        <path d="M518 199H947V263H518ZM518 455H947V471H518Z" fill="#263239" />
        <path
          d="M522 199H947M522 263H947M522 455H947"
          stroke="#c4d2d6"
          strokeWidth="1.5"
        />
        <path d="M522 202H947M522 259H947" stroke="#172126" strokeWidth="2" />
        <path d="M522 149V199M522 263V455" stroke="#bdc9cd" strokeWidth="2" />

        <g transform="translate(581.6 359) scale(8)">
          <circle r="5.45" fill="#263239" stroke="#c2cdd0" strokeWidth=".7" />
          <circle r="3.25" fill="#76878b" />
          <polygon
            points="2,0 1,1.73205 -1,1.73205 -2,0 -1,-1.73205 1,-1.73205"
            fill="#152024"
          />
        </g>
        <circle
          cx="581.6"
          cy="359"
          r="13"
          fill="none"
          stroke="#a0f8f7"
          strokeWidth="2"
        />
        <path
          d="M581.6 341V377M563.6 359H599.6"
          stroke="#a0f8f7"
          strokeWidth="1.5"
        />
        <path
          d="M610 354L604 359 610 364M604 359H663"
          fill="none"
          stroke="#a0f8f7"
          strokeWidth="2"
        />
        <text x="674" y="366" fontSize="22" fontWeight="750">
          {t("Probe ball here")}
        </text>
      </svg>

      <svg x="470" y="38" width="454" height="286" viewBox="0 0 454 286">
        <defs>
          <pattern
            id={hatch}
            width="12"
            height="12"
            patternUnits="userSpaceOnUse"
          >
            <rect width="12" height="12" fill="#546970" />
            <path
              d="M-1 1L1 -1M0 12L12 0M11 13L13 11"
              stroke="#354b53"
              strokeWidth="1.5"
            />
          </pattern>
        </defs>
        <rect width="454" height="286" fill="#202b30" />
        <path
          d="M0 92H151L155 96V255H184V286H0ZM454 92H303L299 96V255H270V286H454Z"
          fill={`url(#${hatch})`}
        />
        <path
          d="M0 92H151L155 96V255H184V286M454 92H303L299 96V255H270V286"
          fill="none"
          stroke="#b6c6cb"
          strokeWidth="2"
        />
        <path
          d="M162 187L166 183H194V213H260V183H288L292 187V250L287 255H266V286H188V255H167L162 250Z"
          fill="#88979e"
          stroke="#b6c6cb"
          strokeWidth="2"
        />
        <rect x="220.5" y="12" width="13" height="119" rx="2" fill="#f2f4f2" />
        <path d="M231 15V122" stroke="#aebfc7" strokeWidth="2" />
        <circle
          cx="227"
          cy="130"
          r="13"
          fill="#bb3153"
          stroke="#fb7696"
          strokeWidth="1.5"
        />
        <ellipse
          cx="223"
          cy="125"
          rx="4"
          ry="2.5"
          fill="#ffafc1"
          transform="rotate(-34 223 125)"
        />
      </svg>
    </svg>
  );
}
