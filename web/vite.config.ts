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

import { defineConfig } from "vite";
import { cpSync, existsSync, mkdirSync, readFileSync } from "node:fs";
import { resolve, extname } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const media = {
  animations: "web/assets/animations",
  docs: "docs",
  fonts: "web/assets/fonts",
};
export default defineConfig({
  server: { port: 8140, proxy: { "/api": "http://127.0.0.1:8137" } },
  plugins: [
    {
      name: "probe-media",
      configureServer(server) {
        server.middlewares.use("/media", (request, response, next) => {
          const [group, ...parts] = decodeURIComponent(
            (request.url || "").split("?")[0],
          )
            .split("/")
            .filter(Boolean);
          const directory = media[group as keyof typeof media];
          if (!directory) return next();
          const base = resolve(root, directory);
          const path = resolve(base, ...parts);
          if (!path.startsWith(base + "/") || !existsSync(path)) return next();
          const type = {
            ".png": "image/png",
            ".svg": "image/svg+xml",
            ".otf": "font/otf",
            ".jpg": "image/jpeg",
          }[extname(path)];
          if (type) response.setHeader("Content-Type", type);
          response.end(readFileSync(path));
        });
      },
      closeBundle() {
        for (const [group, directory] of Object.entries(media)) {
          if (!existsSync(resolve(root, directory))) continue;
          mkdirSync(resolve(root, "web/dist/media"), { recursive: true });
          cpSync(
            resolve(root, directory),
            resolve(root, "web/dist/media", group),
            { recursive: true },
          );
        }
      },
    },
  ],
});
