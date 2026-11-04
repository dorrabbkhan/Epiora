// Static file server for the bench: serves crates/epiora-wasm (the
// parent directory) so /pkg/epiora_wasm.js and /bench/bench.html are
// reachable. Started by Playwright's webServer (playwright.config.mjs).
//
//   node serve.mjs [port]

import http from "node:http";
import { promises as fs } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const port = Number(process.argv[2] ?? 8765);

const MIME = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".mjs": "text/javascript",
  ".wasm": "application/wasm",
  ".json": "application/json",
  ".map": "application/json",
};

http
  .createServer((req, res) => {
    const urlPath = decodeURIComponent(new URL(req.url, "http://x").pathname);
    if (urlPath === "/") {
      res.writeHead(200, { "content-type": "text/plain" });
      res.end("epiora-wasm bench server\n");
      return;
    }
    const file = path.join(root, urlPath);
    if (!file.startsWith(root + path.sep)) {
      res.writeHead(403).end("forbidden");
      return;
    }
    fs.readFile(file)
      .then((data) => {
        res.writeHead(200, {
          "content-type": MIME[path.extname(file)] ?? "application/octet-stream",
        });
        res.end(data);
      })
      .catch(() => {
        res.writeHead(404).end(`not found: ${urlPath}`);
      });
  })
  .listen(port, "127.0.0.1", () => {
    console.log(`serving ${root} on http://127.0.0.1:${port}`);
  });
