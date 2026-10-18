#!/usr/bin/env node
import { bumpSemver, loadManifest, parsePackage, saveManifest, syncPackage } from "./lib.mjs";

const [name, part, explicit] = process.argv.slice(2);
if (!name || (!part && !explicit)) {
  console.error("usage: node scripts/version/bump.mjs <web|desktop> <patch|minor|major>");
  console.error("       node scripts/version/bump.mjs <web|desktop> set <x.y.z>");
  process.exit(1);
}

const pkg = parsePackage(name);
const manifest = loadManifest();
let next;
if (part === "set") {
  if (!explicit || !/^\d+\.\d+\.\d+$/.test(explicit)) {
    console.error("usage: bump.mjs <web|desktop> set <x.y.z>");
    process.exit(1);
  }
  next = explicit;
} else {
  next = bumpSemver(pkg.version, part);
}

manifest.packages[name].version = next;
saveManifest(manifest);
syncPackage(name);
console.log(`${name}: ${pkg.version} -> ${next}`);
