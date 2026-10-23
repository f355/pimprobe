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

import { createContext, useContext, useMemo, type ReactNode } from "react";
import swedish from "./locales/sv.json" with { type: "json" };
import chinese from "./locales/zh_CN.json" with { type: "json" };

export type Language = "en" | "zh_CN" | "sv";
export type Translate = (
  source: string,
  values?: (string | number)[],
) => string;

const catalogs: Record<Language, Record<string, string>> = {
  en: {},
  sv: swedish,
  zh_CN: chinese,
};
type Template = { source: string; groups: number[]; expression: RegExp };
const templates = new Map<Language, Template[]>();

function message(language: Language, source: string): string {
  if (language === "en") return source;
  const catalog = catalogs[language];
  if (Object.hasOwn(catalog, source)) return catalog[source];
  if (source.includes("\n"))
    return source
      .split("\n")
      .map((line) => message(language, line))
      .join("\n");
  if (!templates.has(language)) {
    templates.set(
      language,
      Object.keys(catalog)
        .filter((key) => /%[1-9]\d*/.test(key))
        .sort((a, b) => b.length - a.length)
        .map((source) => {
          const groups: number[] = [];
          let pattern = "^",
            offset = 0;
          const escape = (text: string) =>
            text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
          for (const match of source.matchAll(/%([1-9]\d*)/g)) {
            pattern += escape(source.slice(offset, match.index));
            const number = Number(match[1]),
              group = groups.indexOf(number);
            if (group >= 0) pattern += "\\" + (group + 1) + "(?:)";
            else {
              groups.push(number);
              pattern += "(.+?)";
            }
            offset = match.index + match[0].length;
          }
          return {
            source,
            groups,
            expression: new RegExp(
              pattern + escape(source.slice(offset)) + "$",
            ),
          };
        }),
    );
  }
  // Service messages arrive formatted; translate their wrappers, not coordinates or error codes.
  for (const template of templates.get(language)!) {
    const match = template.expression.exec(source);
    if (!match) continue;
    const values: string[] = [];
    template.groups.forEach((number, index) => {
      values[number - 1] = message(language, match[index + 1]);
    });
    return translate(language, template.source, values);
  }
  return source;
}

export function translate(
  language: Language,
  source: string,
  values?: (string | number)[],
): string {
  if (!values) return message(language, source);
  const catalog = catalogs[language];
  const text = Object.hasOwn(catalog, source) ? catalog[source] : source;
  return text.replace(/%([1-9]\d*)/g, (match, number: string) => {
    const value = values?.[Number(number) - 1];
    return value === undefined ? match : String(value);
  });
}

const I18nContext = createContext<{ language: Language; t: Translate }>({
  language: "en",
  t: (source, values) => translate("en", source, values),
});

export function I18nProvider({
  language,
  children,
}: {
  language: string;
  children: ReactNode;
}) {
  const name = (
    language || (typeof navigator !== "undefined" ? navigator.language : "en")
  )
    .replace(/-/g, "_")
    .toLowerCase();
  const supported: Language =
    name === "zh" || name.startsWith("zh_cn") || name.startsWith("zh_hans")
      ? "zh_CN"
      : name === "sv" || name.startsWith("sv_")
        ? "sv"
        : "en";
  const value = useMemo(
    () => ({
      language: supported,
      t: (source: string, values?: (string | number)[]) =>
        translate(supported, source, values),
    }),
    [supported],
  );
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useTranslation() {
  return useContext(I18nContext);
}
