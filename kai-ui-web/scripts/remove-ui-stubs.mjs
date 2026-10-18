#!/usr/bin/env node
/**
 * Elimina carpetas stub de primitivos UI tras migrar imports a @kai/ui.
 * Uso: node packages/ui/scripts/remove-ui-stubs.mjs [--dry-run] [app|all]
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { execSync } from "node:child_process";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const dryRun = process.argv.includes("--dry-run");
const cliArgs = process.argv.slice(2).filter((a) => !a.startsWith("-"));
const target = cliArgs[0] ?? "all";

const STUB_FOLDERS = {
  "kai-admin": [
    "src/shared/components/Alert",
    "src/shared/components/Badge",
    "src/shared/components/Button",
    "src/shared/components/IconButton",
    "src/shared/components/Switch",
    "src/shared/components/DotProgress",
    "src/shared/components/Select",
    "src/shared/components/AutoComplete",
    "src/shared/components/DropdownList",
    "src/shared/components/NumberStepper",
    "src/shared/components/TextField",
    "src/shared/components/Dialog",
    "src/shared/components/DataGrid",
    "src/shared/components/Tabs",
    "src/shared/components/Cards",
    "src/shared/components/Stepper",
    "src/shared/components/RangeSlider",
    "src/shared/components/LoadingState",
    "src/shared/components/Skeleton",
    "src/shared/components/PrintDialog",
    "src/shared/components/layouts",
  ],
  "kai-pos": [
    "src/shared/components/Alert",
    "src/shared/components/Badge",
    "src/shared/components/Button",
    "src/shared/components/IconButton",
    "src/shared/components/Switch",
    "src/shared/components/DotProgress",
    "src/shared/components/Select",
    "src/shared/components/AutoComplete",
    "src/shared/components/DropdownList",
    "src/shared/components/NumberStepper",
    "src/shared/components/TextField",
    "src/shared/components/Dialog",
    "src/shared/components/DataGrid",
    "src/shared/components/PrintDialog",
    "src/shared/hooks/useCoarsePointer.ts",
  ],
  "kai-eshop": [
    "src/shared/components/Alert",
    "src/shared/components/Badge",
    "src/shared/components/Button",
    "src/shared/components/IconButton",
    "src/shared/components/Switch",
    "src/shared/components/DotProgress",
    "src/shared/components/Select",
    "src/shared/components/DropdownList",
    "src/shared/components/TextField",
    "src/shared/components/Dialog",
    "src/shared/components/Tabs",
    "src/shared/components/Skeleton",
    "src/shared/components/LoadingState",
  ],
  "kai-stock": [
    "src/shared/Alert",
    "src/shared/Badge",
    "src/shared/Button",
    "src/shared/IconButton",
    "src/shared/Switch",
    "src/shared/DotProgress",
    "src/shared/Select",
    "src/shared/DropdownList",
    "src/shared/NumberStepper",
    "src/shared/TextField",
    "src/shared/Dialog",
    "src/shared/LoadingState",
    "src/shared/components/Cards",
  ],
};

const BRIDGE_FILES = {
  "kai-pos": ["src/shared/admin-shared.ts"],
  "kai-eshop": ["src/shared/admin-shared.ts"],
};

function hasReferences(app, folderName) {
  const appRoot = path.join(root, app);
  const scanRoots = app === "kai-admin" ? ["app", "src"] : ["src"];
  const patterns = [`@/shared/components/${folderName}`, `@/shared/${folderName}`];
  for (const scanRoot of scanRoots) {
    const dir = path.join(appRoot, scanRoot);
    if (!fs.existsSync(dir)) continue;
    try {
      for (const pattern of patterns) {
        const result = execSync(
          `rg -l --glob '*.{ts,tsx}' "${pattern.replace(/"/g, '\\"')}" "${dir}" 2>/dev/null || true`,
          { encoding: "utf8", cwd: root },
        ).trim();
        if (result) {
          const files = result
            .split("\n")
            .filter(Boolean)
            .filter(
              (f) =>
                !f.includes(`/shared/${folderName}/`) &&
                !f.includes(`/shared/components/${folderName}/`),
            );
          if (files.length) return true;
        }
      }
    } catch {
      // rg error — skip gate
    }
  }
  return false;
}

function removePath(relPath, app) {
  const full = path.join(root, app, relPath);
  if (!fs.existsSync(full)) return false;
  if (dryRun) {
    console.log(`would remove: ${app}/${relPath}`);
    return true;
  }
  fs.rmSync(full, { recursive: true, force: true });
  console.log(`removed: ${app}/${relPath}`);
  return true;
}

const apps = target === "all" ? Object.keys(STUB_FOLDERS) : [target];

for (const app of apps) {
  const folders = STUB_FOLDERS[app] ?? [];
  for (const folder of folders) {
    const folderName = path.basename(folder).replace(/\.tsx?$/, "");
    if (hasReferences(app, folderName)) {
      console.warn(`SKIP ${app}/${folder}: still has references`);
      continue;
    }
    removePath(folder, app);
  }

  for (const bridge of BRIDGE_FILES[app] ?? []) {
    const bridgeName = path.basename(bridge, ".ts");
    try {
      const scanRoot = path.join(root, app, "src");
      const result = execSync(
        `rg -l --glob '*.{ts,tsx}' "@/shared/${bridgeName}" "${scanRoot}" 2>/dev/null || true`,
        { encoding: "utf8" },
      ).trim();
      const files = result.split("\n").filter(Boolean).filter((f) => !f.endsWith(bridge));
      if (files.length) {
        console.warn(`SKIP ${app}/${bridge}: still has ${files.length} references`);
        continue;
      }
    } catch {
      // continue
    }
    removePath(bridge, app);
  }
}
