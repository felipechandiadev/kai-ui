#!/usr/bin/env node
import { syncAll, loadManifest, readInstalledVersion } from "./lib.mjs";

syncAll();
const manifest = loadManifest();
for (const name of Object.keys(manifest.packages)) {
  const expected = manifest.packages[name].version;
  const actual = readInstalledVersion(name);
  if (expected !== actual) {
    console.error(`${name}: version.json has ${expected} but manifest has ${actual}`);
    process.exit(1);
  }
}
console.log("versions synced:", Object.fromEntries(
  Object.keys(manifest.packages).map((n) => [n, manifest.packages[n].version]),
));
