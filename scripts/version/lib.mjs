import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "../..");
const VERSION_FILE = join(ROOT, "version.json");

export function repoRoot() {
  return ROOT;
}

export function loadManifest() {
  return JSON.parse(readFileSync(VERSION_FILE, "utf8"));
}

export function saveManifest(manifest) {
  writeFileSync(VERSION_FILE, `${JSON.stringify(manifest, null, 2)}\n`);
}

export function parsePackage(name) {
  const manifest = loadManifest();
  const pkg = manifest.packages?.[name];
  if (!pkg) {
    throw new Error(`unknown package "${name}"; expected web | desktop`);
  }
  return { name, ...pkg };
}

export function bumpSemver(version, part) {
  const match = /^(\d+)\.(\d+)\.(\d+)(?:-.+)?$/.exec(version);
  if (!match) {
    throw new Error(`invalid semver: ${version}`);
  }
  let [major, minor, patch] = match.slice(1, 4).map(Number);
  switch (part) {
    case "major":
      major += 1;
      minor = 0;
      patch = 0;
      break;
    case "minor":
      minor += 1;
      patch = 0;
      break;
    case "patch":
      patch += 1;
      break;
    default:
      throw new Error(`bump part must be major | minor | patch, got "${part}"`);
  }
  return `${major}.${minor}.${patch}`;
}

export function readWebVersion() {
  const path = join(ROOT, "kai-ui-web/package.json");
  const pkg = JSON.parse(readFileSync(path, "utf8"));
  return pkg.version;
}

export function readDesktopVersion() {
  const path = join(ROOT, "kai-ui-desktop/Cargo.toml");
  const text = readFileSync(path, "utf8");
  const match = /^version\s*=\s*"([^"]+)"/m.exec(text);
  if (!match) {
    throw new Error("could not parse version from kai-ui-desktop/Cargo.toml");
  }
  return match[1];
}

export function writeWebVersion(version) {
  const path = join(ROOT, "kai-ui-web/package.json");
  const pkg = JSON.parse(readFileSync(path, "utf8"));
  pkg.version = version;
  writeFileSync(path, `${JSON.stringify(pkg, null, 2)}\n`);

  const lockPath = join(ROOT, "kai-ui-web/package-lock.json");
  const lock = JSON.parse(readFileSync(lockPath, "utf8"));
  lock.version = version;
  if (lock.packages?.[""]) {
    lock.packages[""].version = version;
  }
  writeFileSync(lockPath, `${JSON.stringify(lock, null, 2)}\n`);
}

export function writeDesktopVersion(version) {
  const path = join(ROOT, "kai-ui-desktop/Cargo.toml");
  let text = readFileSync(path, "utf8");
  if (!/^version\s*=/m.test(text)) {
    throw new Error("version key missing in Cargo.toml");
  }
  text = text.replace(/^version\s*=\s*"[^"]+"/m, `version = "${version}"`);
  writeFileSync(path, text);

  const lockPath = join(ROOT, "kai-ui-desktop/Cargo.lock");
  let lock = readFileSync(lockPath, "utf8");
  lock = lock.replace(
    /^(name = "kai-ui-desktop-prototype"\nversion = )"[^"]+"/m,
    `$1"${version}"`,
  );
  writeFileSync(lockPath, lock);
}

export function syncPackage(name) {
  const pkg = parsePackage(name);
  if (name === "web") {
    writeWebVersion(pkg.version);
  } else if (name === "desktop") {
    writeDesktopVersion(pkg.version);
  }
}

export function syncAll() {
  for (const name of Object.keys(loadManifest().packages)) {
    syncPackage(name);
  }
}

export function readInstalledVersion(name) {
  if (name === "web") {
    return readWebVersion();
  }
  if (name === "desktop") {
    return readDesktopVersion();
  }
  throw new Error(`unknown package ${name}`);
}

export function tagForPackage(name) {
  const pkg = parsePackage(name);
  return `${pkg.tagPrefix}${pkg.version}`;
}
