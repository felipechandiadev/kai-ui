import { cpSync, mkdirSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";

const sourceRoot = new URL("../src/", import.meta.url);
const distRoot = new URL("../dist/", import.meta.url);

function walk(dir) {
  for (const entry of readdirSync(dir)) {
    const absolute = join(dir, entry);
    if (statSync(absolute).isDirectory()) {
      walk(absolute);
      continue;
    }
    if (!entry.endsWith(".css")) continue;
    const rel = relative(sourceRoot.pathname, absolute);
    const dest = join(distRoot.pathname, rel);
    mkdirSync(dirname(dest), { recursive: true });
    cpSync(absolute, dest);
  }
}

walk(sourceRoot.pathname);
