#!/usr/bin/env node
import { execSync } from "node:child_process";
import { tagForPackage, parsePackage } from "./lib.mjs";

const name = process.argv[2];
if (!name) {
  console.error("usage: node scripts/version/tag.mjs <web|desktop>");
  process.exit(1);
}

const pkg = parsePackage(name);
const tag = tagForPackage(name);
try {
  execSync(`git rev-parse -q --verify "refs/tags/${tag}"`, { stdio: "ignore" });
  console.error(`tag ${tag} already exists`);
  process.exit(1);
} catch {
  // ok
}

execSync(`git tag -a "${tag}" -m "${name} ${pkg.version}"`, { stdio: "inherit" });
console.log(`created tag ${tag}`);
