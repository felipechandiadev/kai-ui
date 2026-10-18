#!/usr/bin/env node
/**
 * Reemplaza implementaciones locales por re-exports desde @kai/ui.
 * ADVERTENCIA: tras la limpieza post-@kai/ui, no ejecutar en apps ya migradas
 * (recrearía stubs eliminados). Usar solo al introducir un primitivo nuevo
 * antes de migrar imports. Ver packages/ui/ADAPTACION.md.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "../../..");

const appConfigs = [
  { app: "kai-admin", componentsRoot: "src/shared/components" },
  { app: "kai-pos", componentsRoot: "src/shared/components" },
  { app: "kai-eshop", componentsRoot: "src/shared/components" },
  { app: "kai-stock", componentsRoot: "src/shared" },
];

const stubs = {
  Alert: {
    "index.ts": `export { Alert as default, type AlertVariant } from "@kai/ui";\n`,
    "Alert.tsx": `export { Alert as default, type AlertVariant } from "@kai/ui";\n`,
  },
  Badge: {
    "index.ts": `export { Badge, type BadgeVariant } from "@kai/ui";\nexport { Badge as default } from "@kai/ui";\n`,
    "Badge.tsx": `export { Badge as default, type BadgeVariant } from "@kai/ui";\n`,
  },
  Button: {
    "index.ts": `export { Button, ButtonPill, ButtonGroup, ButtonGroupItem, ButtonGroupToggle } from "@kai/ui";\nexport type { ButtonGroupProps, ButtonGroupItemProps, ButtonGroupToggleProps, ButtonGroupToggleOption, ButtonGroupDensity } from "@kai/ui";\n`,
    "Button.tsx": `export { Button } from "@kai/ui";\n`,
    "ButtonPill.tsx": `export { ButtonPill } from "@kai/ui";\n`,
    "ButtonGroup.tsx": `export { ButtonGroup, ButtonGroupItem, ButtonGroupToggle } from "@kai/ui";\nexport type { ButtonGroupProps, ButtonGroupItemProps, ButtonGroupToggleProps, ButtonGroupToggleOption, ButtonGroupDensity } from "@kai/ui";\n`,
  },
  IconButton: {
    "index.ts": `export { IconButton as default } from "@kai/ui";\n`,
    "IconButton.tsx": `export { IconButton as default } from "@kai/ui";\n`,
  },
  Switch: {
    "index.ts": `export { Switch as default, type SwitchOptionLabels, type SwitchDensity } from "@kai/ui";\n`,
    "Switch.tsx": `export { Switch as default, type SwitchOptionLabels, type SwitchDensity } from "@kai/ui";\n`,
  },
  DotProgress: {
    "index.ts": `export { DotProgress as default } from "@kai/ui";\n`,
    "DotProgress.tsx": `export { DotProgress as default } from "@kai/ui";\n`,
  },
  Select: {
    "index.ts": `export { Select, type SelectOption, type Option } from "@kai/ui";\nexport { SelectDefault as default } from "@kai/ui";\n`,
    "Select.tsx": `export { SelectDefault as default, type Option } from "@kai/ui";\n`,
  },
  AutoComplete: {
    "index.ts": `export { AutoComplete as default } from "@kai/ui";\n`,
    "AutoComplete.tsx": `export { AutoComplete as default } from "@kai/ui";\n`,
  },
  DropdownList: {
    "index.ts": `export { DropdownList as default } from "@kai/ui";\n`,
    "DropdownList.tsx": `export { DropdownList as default } from "@kai/ui";\n`,
  },
  NumberStepper: {
    "index.ts": `export { NumberStepper as default } from "@kai/ui";\n`,
    "NumberStepper.tsx": `export { NumberStepper as default } from "@kai/ui";\n`,
  },
  TextField: {
    "index.ts": `export { default, TextField } from "@kai/ui";\n`,
    "TextField.tsx": `export { TextField, default } from "@kai/ui";\n`,
    "resolve-touch-input-mode.ts": `export { resolveTouchInputMode, shouldUseTextInputForNumericType, type TouchInputMode, type ResolveTouchInputModeParams } from "@kai/ui";\n`,
  },
  Dialog: {
    "index.ts": `export { Dialog as default, DeleteDialog, type DeleteDialogProps } from "@kai/ui";\n`,
    "Dialog.tsx": `export { Dialog as default } from "@kai/ui";\n`,
    "DeleteDialog.tsx": `export { DeleteDialog as default, DeleteDialog, type DeleteDialogProps } from "@kai/ui";\n`,
    "ChangePasswordDialog.tsx": null,
  },
  layouts: {
    "index.ts": `export * from "@kai/ui/components/layouts";\n`,
    "BasicPageLayout.tsx": `export { BasicPageLayout, type BasicPageLayoutProps } from "@kai/ui";\n`,
    "CollectionPageLayout.tsx": `export { CollectionPageLayout, buildContentGridClassNames, type CollectionGridColumnConfig, type CollectionPageLayoutProps } from "@kai/ui";\n`,
    "TabPageLayout.tsx": `export { TabPageLayout, type TabPageLayoutProps } from "@kai/ui";\n`,
    "layoutPageTokens.ts": `export { adminFillViewportBelowTopBarClassName, dataGridFillViewportTabPageProps, layoutPageContentClassName, layoutPageHeaderClassName, layoutPageRootClassName, layoutPageRootClassNameCompact, layoutPageSubtitleClassName, layoutPageTitleClassName } from "@kai/ui";\n`,
  },
  DataGrid: {
    "index.ts": `export { DataGrid as default, RowActions } from "@kai/ui";\nexport type { DataGridColumn, DataGridCellOverflow, DataGridProps } from "@kai/ui";\n`,
    "DataGrid.tsx": `export { DataGridTable as default } from "@kai/ui";\nexport type { DataGridColumn, DataGridCellOverflow, DataGridProps } from "@kai/ui";\n`,
    "DataGridWrapper.tsx": `export { DataGrid as default } from "@kai/ui";\n`,
    "components/RowActions.tsx": `export { RowActions } from "@kai/ui";\n`,
  },
  Tabs: {
    "index.ts": `export { Tabs as default, type TabItem, type TabsProps } from "@kai/ui";\n`,
    "Tabs.tsx": `export { Tabs as default, type TabItem, type TabsProps } from "@kai/ui";\n`,
  },
  Cards: {
    "index.ts": `export { Card, StatisticsCard, type CardProps, type StatisticsCardProps } from "@kai/ui";\n`,
    "Card.tsx": `export { Card, type CardProps, type CardAction, type CardIconAction, type CardTextAction, type LucideIconName } from "@kai/ui";\n`,
    "StatisticsCard.tsx": `export { StatisticsCard, type StatisticsCardProps, type StatisticsValueTone } from "@kai/ui";\n`,
  },
  Stepper: {
    "index.ts": `export { Stepper as default, type StepperProps, type StepperStepItem } from "@kai/ui";\n`,
    "Stepper.tsx": `export { Stepper as default, type StepperProps, type StepperStepItem } from "@kai/ui";\n`,
  },
  RangeSlider: {
    "index.ts": `export { RangeSlider as default } from "@kai/ui";\n`,
    "RangeSlider.tsx": `export { RangeSlider as default } from "@kai/ui";\n`,
  },
  LoadingState: {
    "index.ts": `export { LoadingState as default, type LoadingStateProps } from "@kai/ui";\n`,
    "LoadingState.tsx": `export { LoadingState as default, type LoadingStateProps } from "@kai/ui";\n`,
  },
  Skeleton: {
    "index.ts": `export { Skeleton as default, type SkeletonProps } from "@kai/ui";\n`,
    "Skeleton.tsx": `export { Skeleton as default, type SkeletonProps } from "@kai/ui";\n`,
  },
  PrintDialog: {
    "index.ts": `export { PrintDialog, usePrint, type PrintDialogProps, type PageOrientation, type PageSize } from "@kai/ui";\nexport { PrintDialog as default } from "@kai/ui";\n`,
    "PrintDialog.tsx": `export { PrintDialog, type PrintDialogProps } from "@kai/ui";\nexport { PrintDialog as default } from "@kai/ui";\n`,
    "usePrint.ts": `export { usePrint } from "@kai/ui";\n`,
    "PrintDialog.types.ts": `export type { PrintDialogProps, PageOrientation, PageSize } from "@kai/ui";\n`,
  },
};

const stockOnlyStubs = ["Alert", "Badge", "Button", "IconButton", "Switch", "DotProgress", "Select", "DropdownList", "NumberStepper", "TextField", "Dialog", "Cards", "LoadingState"];

function stubFolder(app, componentsRoot, folder, files) {
  const dir = path.join(root, app, componentsRoot, folder);
  if (!fs.existsSync(dir)) {
    return;
  }
  if (folder === "DataGrid") {
    const compDir = path.join(dir, "components");
    if (fs.existsSync(compDir)) {
      fs.rmSync(compDir, { recursive: true, force: true });
    }
  }
  for (const entry of fs.readdirSync(dir)) {
    const full = path.join(dir, entry);
    if (entry === "ChangePasswordDialog.tsx") continue;
    if (fs.statSync(full).isFile()) {
      fs.unlinkSync(full);
    } else if (fs.statSync(full).isDirectory()) {
      fs.rmSync(full, { recursive: true, force: true });
    }
  }
  for (const [file, content] of Object.entries(files)) {
    if (content == null) {
      continue;
    }
    const target = path.join(dir, file);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, content, "utf8");
  }
  console.log(`stubbed ${app}/${componentsRoot}/${folder}`);
}

for (const { app, componentsRoot } of appConfigs) {
  const allowed =
    app === "kai-stock"
      ? stockOnlyStubs
      : Object.keys(stubs).filter((k) => k !== "layouts" || app !== "kai-eshop");
  for (const folder of allowed) {
    if (!stubs[folder]) continue;
    if (folder === "layouts" && app === "kai-eshop") continue;
    if (folder === "layouts" && app === "kai-stock") continue;
    if (folder === "DataGrid" && (app === "kai-eshop" || app === "kai-stock")) continue;
    if (folder === "AutoComplete" && app === "kai-stock") continue;
    if (folder === "PrintDialog" && app === "kai-eshop") continue;
    if (folder === "Stepper" && (app === "kai-eshop" || app === "kai-stock")) continue;
    if (folder === "RangeSlider" && (app === "kai-eshop" || app === "kai-stock")) continue;
    if (folder === "Tabs" && app === "kai-stock") continue;
    stubFolder(app, componentsRoot, folder, stubs[folder]);
  }
}

// Stock Cards live under components/Cards
if (fs.existsSync(path.join(root, "kai-stock/src/shared/components/Cards"))) {
  stubFolder("kai-stock", "src/shared/components", "Cards", stubs.Cards);
}

const posHook = path.join(root, "kai-pos/src/shared/hooks/useCoarsePointer.ts");
if (fs.existsSync(posHook)) {
  fs.writeFileSync(posHook, `export { useCoarsePointer } from "@kai/ui";\n`, "utf8");
}

const posAdminShared = path.join(root, "kai-pos/src/shared/admin-shared.ts");
fs.writeFileSync(
  posAdminShared,
  `// Re-exports desde @kai/ui (fuente: packages/ui)
export {
  Dialog,
  Alert,
  Button,
  TextField,
  DotProgress,
  Select,
  IconButton,
  NumberStepper,
  Switch,
  DataGrid,
  Tabs,
  Card,
  StatisticsCard,
  LoadingState,
  Skeleton,
} from "@kai/ui";
export type { SwitchOptionLabels, DataGridColumn, TabItem, TabsProps } from "@kai/ui";
`,
  "utf8",
);

const eshopAdminShared = path.join(root, "kai-eshop/src/shared/admin-shared.ts");
if (fs.existsSync(eshopAdminShared)) {
  fs.writeFileSync(
    eshopAdminShared,
    `// Re-exports desde @kai/ui (fuente: packages/ui)
export {
  Button,
  TextField,
  IconButton,
  Select,
  Dialog,
  Alert,
  DotProgress,
  Switch,
  Tabs,
  Skeleton,
  LoadingState,
} from "@kai/ui";
export type { Option as SelectOption, TabItem, TabsProps } from "@kai/ui";
`,
    "utf8",
  );
}

const stockPageLoading = path.join(root, "kai-stock/src/shared/components/PageLoading.tsx");
if (fs.existsSync(stockPageLoading)) {
  fs.writeFileSync(
    stockPageLoading,
    `"use client";

import { LoadingState } from "@kai/ui";

export default function PageLoading() {
  return <LoadingState className="flex justify-center py-12" />;
}
`,
    "utf8",
  );
}

console.log("done");
