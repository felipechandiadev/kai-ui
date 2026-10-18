#!/usr/bin/env node
import { loadManifest, readInstalledVersion } from "./lib.mjs";

const manifest = loadManifest();
let failed = false;
for (const name of Object.keys(manifest.packages)) {
  const expected = manifest.packages[name].version;
  const actual = readInstalledVersion(name);
  if (expected !== actual) {
    console.error(
      `[version] ${name}: version.json=${expected} manifest=${actual} — run: node scripts/version/sync.mjs`,
    );
    failed = true;
  }
}
if (failed) {
  process.exit(1);
}
console.log(
  "version check ok:",
  Object.fromEntries(
    Object.keys(manifest.packages).map((n) => [n, manifest.packages[n].version]),
  ),
);
