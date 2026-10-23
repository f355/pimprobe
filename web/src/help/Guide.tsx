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

import {
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { ChevronRight } from "lucide-react";
import { PageHeader } from "../controls";
import { useTranslation } from "../i18n";
import {
  guideImage,
  guideLink,
  guideNames,
  parseGuide,
  type GuideLocation,
} from "./document";
import "./help.css";

const sources = import.meta.glob<string>(
  ["../../../docs/*.md", "../../../docs/zh_CN/*.md", "../../../docs/sv/*.md"],
  { query: "?raw", import: "default", eager: true },
);
const images = import.meta.glob<string>(
  "../../../docs/images/**/*.{svg,png,jpg,jpeg,gif,webp}",
  { query: "?url", import: "default", eager: true },
);

export interface HelpGuideProps {
  page?: string;
  onBack: () => void;
  onTitle?: (title: string) => void;
}

export function HelpGuide({ page, onBack, onTitle }: HelpGuideProps) {
  const { language, t } = useTranslation();
  const [location, setLocation] = useState<GuideLocation | null>(() =>
    page ? guideLink(page, "README.md") : null,
  );
  const root = useRef<HTMLElement>(null),
    reader = useRef<HTMLDivElement>(null);
  const prefix = useId();
  const guides = useMemo(
    () =>
      guideNames.map((name) => {
        const english = sources["../../../docs/" + name] || "";
        const localized =
          language === "en"
            ? english
            : sources[`../../../docs/${language}/${name}`] || english;
        return { name, ...parseGuide(localized) };
      }),
    [language],
  );
  const current = guides.find((guide) => guide.name === location?.page);
  const title = current?.title || t("Operator Guide");

  useEffect(() => {
    setLocation(page ? guideLink(page, "README.md") : null);
  }, [page]);
  useEffect(() => {
    onTitle?.(title);
  }, [title, onTitle]);
  useLayoutEffect(() => {
    if (!reader.current) return;
    reader.current.scrollTop = 0;
    if (location?.anchor) {
      const section = [
        ...reader.current.querySelectorAll<HTMLDetailsElement>(
          ".pp-help-section",
        ),
      ].find((element) => element.dataset.section === location.anchor);
      if (section) {
        section.open = true;
        reader.current.scrollTop = section.offsetTop;
      }
    }
    const heading = root.current?.querySelector("h1");
    if (heading) {
      heading.tabIndex = -1;
      heading.focus({ preventScroll: true });
    }
  }, [location, language]);

  function markdown(text: string) {
    return (
      <Markdown
        remarkPlugins={[remarkGfm]}
        skipHtml
        components={{
          a: ({ href, children }) => {
            const target =
              href && guideLink(href, current?.name || "README.md");
            return target ? (
              <a
                href={href}
                onClick={(event) => {
                  event.preventDefault();
                  setLocation(target);
                }}
              >
                {children}
              </a>
            ) : (
              <span>{children}</span>
            );
          },
          img: ({ src, alt, title }) => {
            const path = typeof src === "string" ? guideImage(src) : undefined;
            const image = path ? images["../../../docs/" + path] : undefined;
            return image ? (
              <img src={image} alt={alt || ""} title={title} />
            ) : (
              <span>{alt}</span>
            );
          },
          table: ({ children }) => (
            <div className="pp-help-table">
              <table>{children}</table>
            </div>
          ),
        }}
      >
        {text}
      </Markdown>
    );
  }

  return (
    <section
      ref={root}
      className="pp-help-guide"
      aria-label={title}
      lang={language.replace("_", "-")}
    >
      <PageHeader
        title={title}
        onBack={() => (current ? setLocation(null) : onBack())}
      />
      {current ? (
        <div
          ref={reader}
          className="pp-help-reader"
          key={`${language}:${current.name}`}
        >
          {current.introduction && (
            <div className="pp-help-markdown pp-help-introduction">
              {markdown(current.introduction)}
            </div>
          )}
          {current.sections.map((section) => (
            <details
              key={section.id}
              id={`${prefix}-${section.id}`}
              className="pp-help-section"
              data-section={section.id}
            >
              <summary>
                <span>{section.title}</span>
                <ChevronRight size={24} aria-hidden="true" />
              </summary>
              <div className="pp-help-markdown">
                {markdown(section.markdown)}
              </div>
            </details>
          ))}
        </div>
      ) : (
        <nav className="pp-help-index" aria-label={title}>
          {guides.map((guide) => (
            <button
              type="button"
              key={guide.name}
              className="pp-help-page-link"
              onClick={() => setLocation({ page: guide.name, anchor: "" })}
            >
              <span>{guide.title}</span>
              <ChevronRight size={24} aria-hidden="true" />
            </button>
          ))}
        </nav>
      )}
    </section>
  );
}
