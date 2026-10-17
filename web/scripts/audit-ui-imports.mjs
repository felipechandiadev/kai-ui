#!/usr/bin/env node
/**
 * Inventario de imports legacy (@/shared/...) vs @kai/ui por PWA.
 * Uso: node packages/ui/scripts/audit-ui-imports.mjs [--json]
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const jsonOutput = process.argv.includes("--json");

const PRIMITIVE_STUBS = [
  "Alert",
  "Badge",
  "Button",
  "IconButton",
  "Switch",
  "DotProgress",
  "Select",
  "AutoComplete",
  "DropdownList",
  "NumberStepper",
  "TextField",
  "Dialog",
  "DataGrid",
  "Tabs",
  "Cards",
  "Stepper",
  "RangeSlider",
  "LoadingState",
  "Skeleton",
  "PrintDialog",
  "layouts",
];

const DOMAIN_ALLOWLIST = {
  "kai-admin": new Set([
    "TopBar",
    "Multimedia",
    "PurchaseDocumentBuilder",
    "PrintDocuments",
    "PlannedPaymentLines",
    "FileUploader",
    "BaseForm",
    "Calendar",
    "SplashScreen",
    "LocationPicker",
    "StockThresholdField",
    "MercadoPagoLogo.tsx",
    "ErpPlaceholderPage.tsx",
    "types.ts",
    "README.md",
    "INTEGRATION_GUIDE.md",
    "index.ts",
  ]),
  "kai-pos": new Set([
    "PosTopBar",
    "PurchaseDocumentBuilder",
    "PrintDocuments",
    "PlannedPaymentLines",
    "MercadoPagoLogo.tsx",
    "index.ts",
  ]),
  "kai-eshop": new Set([
    "MercadoPagoLogo.tsx",
    "index.ts",
  ]),
  "kai-stock": new Set([
    "StockTopBar",
    "SideBar",
    "StockPageShell",
    "StockAuthenticatedShell",
    "LoginPageShell",
    "PageLoading.tsx",
    "index.ts",
    "hooks",
  ]),
};

const APP_CONFIGS = [
  {
    app: "kai-admin",
    srcRoot: ".",
    scanDirs: ["app", "src"],
    componentsRoots: ["src/shared/components"],
    legacyPatterns: [
      /@\/shared\/components\/([A-Za-z][\w/]*)/g,
      /from ['"]@\/shared\/components['"]/g,
    ],
  },
  {
    app: "kai-pos",
    srcRoot: "src",
    scanDirs: ["src"],
    componentsRoots: ["src/shared/components"],
    legacyPatterns: [
      /@\/shared\/components\/([A-Za-z][\w/]*)/g,
      /from ['"]@\/shared\/components['"]/g,
      /from ['"]@\/shared\/admin-shared['"]/g,
    ],
  },
  {
    app: "kai-eshop",
    srcRoot: "src",
    scanDirs: ["src"],
    componentsRoots: ["src/shared/components"],
    legacyPatterns: [
      /@\/shared\/components\/([A-Za-z][\w/]*)/g,
      /from ['"]@\/shared\/components['"]/g,
      /from ['"]@\/shared\/admin-shared['"]/g,
    ],
  },
  {
    app: "kai-stock",
    srcRoot: "src",
    scanDirs: ["src"],
    componentsRoots: ["src/shared", "src/shared/components"],
    legacyPatterns: [
      /@\/shared\/([A-Za-z][\w/]*)/g,
      /from ['"]@\/shared['"]/g,
    ],
  },
];

const SOURCE_EXTENSIONS = new Set([".ts", ".tsx", ".js", ".jsx", ".md"]);

function walkFiles(dir, out = []) {
  if (!fs.existsSync(dir)) return out;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === "node_modules" || entry.name === ".next") continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walkFiles(full, out);
    else if (SOURCE_EXTENSIONS.has(path.extname(entry.name))) out.push(full);
  }
  return out;
}

function isStubFile(filePath) {
  const content = fs.readFileSync(filePath, "utf8").trim();
  if (!content) return false;
  const lines = content.split("\n").filter((l) => l.trim() && !l.trim().startsWith("//"));
  if (lines.length > 3) return false;
  return lines.every((line) => line.includes("@kai/ui") || line.startsWith('"use client"'));
}

function topLevelName(relativePath) {
  const parts = relativePath.split(path.sep).filter(Boolean);
  return parts[0]?.replace(/\.tsx?$/, "") ?? relativePath;
}

function isEshopDomain(name) {
  return name.startsWith("EShop");
}

function scanApp(config) {
  const appRoot = path.join(root, config.app);
  const scanDirs = config.scanDirs ?? [config.srcRoot];
  const files = scanDirs.flatMap((d) => walkFiles(path.join(appRoot, d)));

  const legacyByComponent = new Map();
  let kaiUiImports = 0;
  let adminSharedImports = 0;
  const legacyFiles = new Set();

  for (const file of files) {
    const rel = path.relative(appRoot, file);
    if (rel.includes("node_modules")) continue;
    const content = fs.readFileSync(file, "utf8");

    if (content.includes('from "@kai/ui"') || content.includes("from '@kai/ui'")) {
      kaiUiImports += 1;
    }
    if (content.includes('from "@/shared/admin-shared"') || content.includes("from '@/shared/admin-shared'")) {
      adminSharedImports += 1;
      legacyFiles.add(rel);
    }

    for (const pattern of config.legacyPatterns) {
      const re = new RegExp(pattern.source, pattern.flags);
      let match;
      while ((match = re.exec(content)) !== null) {
        if (match[1]) {
          const component = match[1].split("/")[0];
          if (!legacyByComponent.has(component)) legacyByComponent.set(component, new Set());
          legacyByComponent.get(component).add(rel);
          legacyFiles.add(rel);
        } else {
          legacyFiles.add(rel);
        }
      }
    }
  }

  const stubCandidates = [];
  const nonStubWarnings = [];

  for (const componentsRoot of config.componentsRoots) {
    const componentsDir = path.join(appRoot, componentsRoot);
    if (!fs.existsSync(componentsDir)) continue;

    for (const entry of fs.readdirSync(componentsDir, { withFileTypes: true })) {
      const name = entry.name;
      if (!entry.isDirectory() && !name.endsWith(".tsx")) continue;
      if (entry.isFile() && !name.endsWith(".tsx")) continue;

      const allowlist = DOMAIN_ALLOWLIST[config.app] ?? new Set();
      const baseName = entry.isDirectory() ? name : name.replace(/\.tsx$/, "");

      if (config.app === "kai-eshop" && isEshopDomain(baseName)) continue;
      if (allowlist.has(name) || allowlist.has(baseName)) continue;

      const folderPath = entry.isDirectory()
        ? path.join(componentsDir, name)
        : path.join(componentsDir, name);

      if (entry.isDirectory()) {
        const filesInFolder = walkFiles(folderPath);
        const allStub =
          filesInFolder.length > 0 && filesInFolder.every((f) => isStubFile(f) || f.endsWith(".css"));
        const refs = legacyByComponent.get(name)?.size ?? 0;

        if (PRIMITIVE_STUBS.includes(name) || allStub) {
          stubCandidates.push({
            folder: path.relative(appRoot, folderPath),
            references: refs,
            allStub,
            deletable: refs === 0 && allStub,
          });
        } else if (!allowlist.has(name)) {
          nonStubWarnings.push({
            folder: path.relative(appRoot, folderPath),
            reason: "not recognized as stub or domain",
          });
        }
      } else if (name.endsWith(".tsx") && !allowlist.has(name)) {
        const stub = isStubFile(folderPath);
        if (!stub && !isEshopDomain(baseName)) {
          nonStubWarnings.push({ file: path.relative(appRoot, folderPath), reason: "standalone tsx" });
        }
      }
    }
  }

  const primitiveLegacy = {};
  for (const primitive of PRIMITIVE_STUBS) {
    const refs = legacyByComponent.get(primitive);
    if (refs?.size) primitiveLegacy[primitive] = refs.size;
  }

  return {
    app: config.app,
    kaiUiImportFiles: kaiUiImports,
    adminSharedImportFiles: adminSharedImports,
    legacyImportFiles: legacyFiles.size,
    primitiveLegacyRefs: primitiveLegacy,
    stubCandidates: stubCandidates.sort((a, b) => a.folder.localeCompare(b.folder)),
    deletableStubs: stubCandidates.filter((s) => s.deletable),
    nonStubWarnings,
  };
}

const reports = APP_CONFIGS.map(scanApp);

if (jsonOutput) {
  console.log(JSON.stringify(reports, null, 2));
  process.exit(0);
}

console.log("=== @kai/ui import audit ===\n");

for (const report of reports) {
  console.log(`## ${report.app}`);
  console.log(`  @kai/ui files:        ${report.kaiUiImportFiles}`);
  console.log(`  legacy import files:  ${report.legacyImportFiles}`);
  if (report.adminSharedImportFiles) {
    console.log(`  admin-shared files:   ${report.adminSharedImportFiles}`);
  }

  const primitives = Object.entries(report.primitiveLegacyRefs);
  if (primitives.length) {
    console.log("  primitive legacy refs:");
    for (const [name, count] of primitives.sort((a, b) => b[1] - a[1])) {
      console.log(`    ${name}: ${count} files`);
    }
  } else {
    console.log("  primitive legacy refs: none");
  }

  if (report.deletableStubs.length) {
    console.log("  deletable stubs (0 refs):");
    for (const s of report.deletableStubs) {
      console.log(`    - ${s.folder}`);
    }
  }

  if (report.nonStubWarnings.length) {
    console.log("  warnings:");
    for (const w of report.nonStubWarnings.slice(0, 5)) {
      console.log(`    - ${w.folder ?? w.file}: ${w.reason}`);
    }
    if (report.nonStubWarnings.length > 5) {
      console.log(`    ... +${report.nonStubWarnings.length - 5} more`);
    }
  }

  console.log("");
}

const totalLegacy = reports.reduce((n, r) => n + r.legacyImportFiles, 0);
console.log(`Total legacy import files across apps: ${totalLegacy}`);
process.exit(totalLegacy > 0 ? 0 : 0);
