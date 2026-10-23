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

export const guideNames = [
  "README.md",
  "outside.md",
  "inside.md",
  "center.md",
  "angle.md",
  "rotary.md",
  "settings.md",
  "results.md",
  "editing.md",
  "history.md",
  "repeatability.md",
] as const;

export interface GuideSection {
  id: string;
  title: string;
  markdown: string;
}
export interface GuideDocument {
  title: string;
  introduction: string;
  sections: GuideSection[];
}
export interface GuideLocation {
  page: string;
  anchor: string;
}

export function guideLink(
  href: string,
  currentPage: string,
): GuideLocation | null {
  if (/^[a-z][\w+.-]*:|^\/\//i.test(href)) return null;
  try {
    const [path, hash = ""] = href.split("#", 2);
    let page =
      decodeURIComponent(path.split("?")[0]).split("/").pop() || currentPage;
    if (!page.endsWith(".md")) page += ".md";
    return (guideNames as readonly string[]).includes(page)
      ? { page, anchor: decodeURIComponent(hash) }
      : null;
  } catch {
    return null;
  }
}

export function guideImage(source: string): string | undefined {
  try {
    const path = decodeURIComponent(source).replace(/^\.\.\//, "");
    if (
      !/^images\/[\w./-]+\.(?:svg|png|jpe?g|gif|webp)$/i.test(path) ||
      path.split("/").some((part) => part === ".." || part === ".")
    )
      return undefined;
    return path;
  } catch {
    return undefined;
  }
}

export function parseGuide(source: string): GuideDocument {
  const markdown = source
    .replace(/\r\n/g, "\n")
    .replace(/<!-- guide-only -->[\s\S]*?<!-- \/guide-only -->/g, "");
  const title = markdown.match(/^# ([^\n]+)\n?/)?.[1] || "";
  const lines = markdown.replace(/^# [^\n]+\n*/, "").split("\n");
  const sections: GuideSection[] = [];
  const introduction: string[] = [];
  const ids = new Map<string, number>();
  let section: GuideSection | undefined;
  let fence = "";
  for (const line of lines) {
    const marker = line.match(/^ {0,3}(`{3,}|~{3,})/);
    if (marker) {
      if (!fence) fence = marker[1];
      else if (marker[1][0] === fence[0] && marker[1].length >= fence.length)
        fence = "";
    }
    const heading = !fence && line.match(/^## (.+?)\s*#*$/);
    if (heading) {
      const title = heading[1];
      const slug = title
        .toLowerCase()
        .replace(/[^\p{L}\p{N}\s_-]/gu, "")
        .trim()
        .replace(/\s+/g, "-");
      const duplicate = ids.get(slug) || 0;
      ids.set(slug, duplicate + 1);
      section = {
        id: duplicate ? `${slug}-${duplicate}` : slug,
        title,
        markdown: "",
      };
      sections.push(section);
    } else if (section) {
      section.markdown += line + "\n";
    } else {
      introduction.push(line);
    }
  }
  return { title, introduction: introduction.join("\n").trim(), sections };
}
