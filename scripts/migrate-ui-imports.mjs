#!/usr/bin/env node
/**
 * Migra imports legacy de primitivos UI hacia @kai/ui.
 * Uso: node packages/ui/scripts/migrate-ui-imports.mjs [--dry-run] [kai-stock|kai-eshop|kai-pos|kai-admin|all]
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const dryRun = process.argv.includes("--dry-run");
const targetArg = process.argv.find((a) => !a.startsWith("-") && a.endsWith(".mjs") === false && a !== "node");
const target = targetArg && !targetArg.includes("/") ? targetArg : "all";

const PRIMITIVE_SYMBOLS = new Set([
  "Alert",
  "Badge",
  "Button",
  "ButtonPill",
  "ButtonGroup",
  "ButtonGroupItem",
  "ButtonGroupToggle",
  "TextField",
  "Switch",
  "Tabs",
  "IconButton",
  "Select",
  "AutoComplete",
  "DropdownList",
  "DataGrid",
  "RowActions",
  "Dialog",
  "DeleteDialog",
  "PrintDialog",
  "RangeSlider",
  "NumberStepper",
  "Stepper",
  "DotProgress",
  "LoadingState",
  "Skeleton",
  "Card",
  "StatisticsCard",
  "BasicPageLayout",
  "CollectionPageLayout",
  "TabPageLayout",
  "buildContentGridClassNames",
  "usePrint",
  "Option",
  "SelectOption",
]);

const PRIMITIVE_TYPES = new Set([
  "AlertVariant",
  "BadgeVariant",
  "ButtonGroupProps",
  "ButtonGroupItemProps",
  "ButtonGroupToggleProps",
  "ButtonGroupToggleOption",
  "ButtonGroupDensity",
  "CardProps",
  "CardAction",
  "CardTextAction",
  "CardIconAction",
  "LucideIconName",
  "StatisticsCardProps",
  "StatisticsValueTone",
  "TabItem",
  "TabsProps",
  "SelectOption",
  "Option",
  "DataGridColumn",
  "DataGridCellOverflow",
  "DataGridProps",
  "DeleteDialogProps",
  "PrintDialogProps",
  "PageOrientation",
  "PageSize",
  "StepperProps",
  "StepperStepItem",
  "LoadingStateProps",
  "SkeletonProps",
  "SwitchOptionLabels",
  "SwitchDensity",
  "BasicPageLayoutProps",
  "CollectionGridColumnConfig",
  "CollectionPageLayoutProps",
  "TabPageLayoutProps",
]);

const PRIMITIVE_FOLDERS = new Set([
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
  "Card",
  "StatisticsCard",
  "Stepper",
  "RangeSlider",
  "LoadingState",
  "Skeleton",
  "PrintDialog",
  "layouts",
  "BasicPageLayout",
  "CollectionPageLayout",
  "TabPageLayout",
  "layoutPageTokens",
  "DeleteDialog",
  "RowActions",
  "usePrint",
  "resolve-touch-input-mode",
  "ButtonPill",
  "ButtonGroup",
  "ButtonGroupItem",
  "ButtonGroupToggle",
]);

const DOMAIN_SEGMENTS = new Set([
  "TopBar",
  "SideBar",
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
  "PosTopBar",
  "MercadoPagoLogo",
  "ErpPlaceholderPage",
  "StockTopBar",
  "StockPageShell",
  "StockAuthenticatedShell",
  "LoginPageShell",
  "PageLoading",
]);

const APP_SCAN_ROOTS = {
  "kai-admin": ["app", "src"],
  "kai-pos": ["src"],
  "kai-eshop": ["src"],
  "kai-stock": ["src"],
};

const DEFAULT_TO_NAMED = new Set([
  "Alert",
  "Dialog",
  "Switch",
  "IconButton",
  "DotProgress",
  "DropdownList",
  "NumberStepper",
  "AutoComplete",
  "TextField",
  "Tabs",
  "RangeSlider",
  "LoadingState",
  "Skeleton",
  "PrintDialog",
  "DataGrid",
]);

function walkFiles(dir, out = []) {
  if (!fs.existsSync(dir)) return out;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === "node_modules" || entry.name === ".next") continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walkFiles(full, out);
    else if (/\.(tsx?|jsx?)$/.test(entry.name)) out.push(full);
  }
  return out;
}

function parseBindingParts(bindings) {
  const parts = [];
  const inner = bindings.trim();
  if (!inner.startsWith("{")) return { allPrimitive: false, primitiveParts: [], domainParts: [] };
  const body = inner.slice(1, inner.lastIndexOf("}"));
  for (const raw of body.split(",")) {
    const p = raw.trim();
    if (!p) continue;
    parts.push(p);
  }
  const primitiveParts = [];
  const domainParts = [];
  for (const p of parts) {
    const name = p.replace(/^type\s+/, "").split(/\s+as\s+/)[0].trim();
    if (PRIMITIVE_SYMBOLS.has(name) || PRIMITIVE_TYPES.has(name)) primitiveParts.push(p);
    else domainParts.push(p);
  }
  return {
    allPrimitive: domainParts.length === 0 && primitiveParts.length > 0,
    primitiveParts,
    domainParts,
  };
}

function isPrimitiveImport(specifier) {
  if (specifier === "@/shared/admin-shared" || specifier === "@/shared") {
    return true;
  }
  if (specifier === "@/shared/components") {
    return "barrel";
  }

  const patterns = [/^@\/shared\/components\/([^/]+)/, /^@\/shared\/([^/]+)/];

  for (const pattern of patterns) {
    const m = specifier.match(pattern);
    if (!m) continue;
    const segment = m[1];
    if (DOMAIN_SEGMENTS.has(segment)) return false;
    if (segment.startsWith("EShop")) return false;
    if (PRIMITIVE_FOLDERS.has(segment)) return true;
  }
  return false;
}

function transformImportLine(line) {
  const importMatch = line.match(/^(\s*import\s+)(.+?)(\s+from\s+)(['"])([^'"]+)\4(.*)$/);
  if (!importMatch) return line;

  const [, prefix, bindings, fromKw, quote, specifier, suffix] = importMatch;
  const primitiveCheck = isPrimitiveImport(specifier);
  if (!primitiveCheck) return line;

  if (primitiveCheck === "barrel") {
    const { allPrimitive, primitiveParts, domainParts } = parseBindingParts(bindings);
    if (allPrimitive) {
      return `${prefix}{ ${primitiveParts.join(", ")} }${fromKw}${quote}@kai/ui${quote}${suffix}`;
    }
    if (primitiveParts.length && domainParts.length) {
      return [
        `${prefix}{ ${primitiveParts.join(", ")} }${fromKw}${quote}@kai/ui${quote}${suffix}`,
        `${prefix}{ ${domainParts.join(", ")} }${fromKw}${quote}@/shared/components${quote}${suffix}`,
      ].join("\n");
    }
    return line;
  }

  let newBindings = bindings.trim();

  if (specifier === "@/shared/admin-shared" || specifier === "@/shared") {
    return `${prefix}${newBindings}${fromKw}${quote}@kai/ui${quote}${suffix}`;
  }

  const defaultOnly = newBindings.match(/^(\w+)$/);
  const defaultAs = newBindings.match(/^(\w+)\s+as\s+(\w+)$/);

  if (defaultOnly && DEFAULT_TO_NAMED.has(defaultOnly[1])) {
    newBindings = `{ ${defaultOnly[1]} }`;
  } else if (defaultAs && DEFAULT_TO_NAMED.has(defaultAs[1])) {
    newBindings = `{ ${defaultAs[1]} as ${defaultAs[2]} }`;
  }

  if (specifier.includes("/Select/Select") && defaultOnly) {
    newBindings = `{ SelectDefault as ${defaultOnly[1]} }`;
  }

  if (specifier.includes("/DataGrid/DataGrid") && defaultOnly) {
    newBindings = `{ DataGridTable as ${defaultOnly[1]} }`;
  }

  if (specifier.includes("/DataGrid/DataGridWrapper") && defaultOnly) {
    newBindings = `{ DataGrid as ${defaultOnly[1]} }`;
  }

  return `${prefix}${newBindings}${fromKw}${quote}@kai/ui${quote}${suffix}`;
}

function mergeKaiUiImports(content) {
  const lines = content.split("\n");
  const importBlocks = [];
  const otherLines = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];
    if (line.trim().startsWith("import ") && line.includes('from "@kai/ui"')) {
      const block = [line];
      i++;
      while (i < lines.length && lines[i].trim().startsWith("import ") && lines[i].includes('from "@kai/ui"')) {
        block.push(lines[i]);
        i++;
      }
      importBlocks.push(block);
    } else {
      otherLines.push({ line, index: otherLines.length });
      i++;
    }
  }

  if (importBlocks.length <= 1) return content;

  const named = new Set();
  const types = new Set();

  for (const block of importBlocks) {
    for (const line of block) {
      const m = line.match(/^import\s+(.+?)\s+from\s+"@kai\/ui"/);
      if (!m) continue;
      const binding = m[1].trim();
      if (binding.startsWith("type ")) {
        types.add(binding.replace(/^type\s+/, "").trim());
        continue;
      }
      if (binding.startsWith("{")) {
        const inner = binding.slice(1, binding.lastIndexOf("}"));
        for (const part of inner.split(",")) {
          const p = part.trim();
          if (!p) continue;
          if (p.startsWith("type ")) types.add(p.replace(/^type\s+/, "").trim());
          else named.add(p);
        }
      } else if (binding.includes(" as ")) {
        named.add(binding);
      } else {
        named.add(binding);
      }
    }
  }

  const namedList = [...named].sort();
  const typeList = [...types].sort();
  const inlineTypes = typeList.map((t) => `type ${t}`);
  const allNamed = [...namedList, ...inlineTypes];
  const merged =
    allNamed.length > 0
      ? `import { ${allNamed.join(", ")} } from "@kai/ui";`
      : `import "@kai/ui";`;
  const firstBlockStart = lines.findIndex((l) => l.includes('from "@kai/ui"'));
  const lastBlockEnd = lines.reduce(
    (last, l, idx) => (l.includes('from "@kai/ui"') ? idx : last),
    firstBlockStart,
  );

  const result = [...lines.slice(0, firstBlockStart), merged, ...lines.slice(lastBlockEnd + 1)];
  return result.join("\n");
}

function transformFile(filePath) {
  const original = fs.readFileSync(filePath, "utf8");
  const lines = original.split("\n");
  let changed = false;

  const transformed = lines
    .flatMap((line) => {
      if (!line.trim().startsWith("import ")) return [line];
      const next = transformImportLine(line);
      if (next !== line) changed = true;
      return next.split("\n");
    })
    .join("\n");

  const merged = mergeKaiUiImports(transformed);
  if (merged !== transformed) changed = true;

  if (changed && !dryRun) {
    fs.writeFileSync(filePath, merged, "utf8");
  }
  return changed;
}

function migrateApp(app) {
  const scanRoots = APP_SCAN_ROOTS[app] ?? ["src"];
  const files = scanRoots.flatMap((r) => walkFiles(path.join(root, app, r)));
  let count = 0;
  for (const file of files) {
    if (file.includes("/shared/components/") && file.match(/\/(Alert|Button|Badge|Dialog)\//)) {
      // skip stub files themselves
      const rel = path.relative(path.join(root, app), file);
      if (rel.match(/shared\/(components\/)?(Alert|Badge|Button|IconButton|Switch|DotProgress|Select|AutoComplete|DropdownList|NumberStepper|TextField|Dialog|DataGrid|Tabs|Cards|Stepper|RangeSlider|LoadingState|Skeleton|PrintDialog|layouts)\//)) {
        continue;
      }
    }
    if (transformFile(file)) {
      count++;
      if (dryRun) console.log(`would update: ${path.relative(root, file)}`);
    }
  }
  console.log(`${app}: ${count} files ${dryRun ? "would be " : ""}updated`);
  return count;
}

const apps = target === "all" ? Object.keys(APP_SCAN_ROOTS) : [target];
for (const app of apps) {
  if (!APP_SCAN_ROOTS[app]) {
    console.error(`Unknown app: ${app}`);
    process.exit(1);
  }
  migrateApp(app);
}
