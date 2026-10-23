#!/usr/bin/env node
// Generates Kai theme token constants from kai-ui-web/src/theme/tokens.css.
//
//   node kai-ui-desktop/scripts/gen-tokens.mjs
//
// Writes src/theme/kai_tokens.rs (versioned in git). Compile does not need
// kai-ui-web or node_modules; only regenerating does.
// Do not edit the generated file by hand.

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const desktop = join(dirname(fileURLToPath(import.meta.url)), "..");
const cssPath = join(desktop, "..", "kai-ui-web", "src", "theme", "tokens.css");
const outPath = join(desktop, "src", "theme", "kai_tokens.rs");

const css = readFileSync(cssPath, "utf8");

/** Strip block comments from the CSS source. */
function stripComments(source) {
  return source.replace(/\/\*[\s\S]*?\*\//g, "");
}

/** Drop @media blocks (responsive density is UNMAPPED). */
function stripMedia(source) {
  let out = "";
  let i = 0;
  while (i < source.length) {
    const at = source.indexOf("@media", i);
    if (at < 0) {
      out += source.slice(i);
      break;
    }
    out += source.slice(i, at);
    let depth = 0;
    let j = at;
    while (j < source.length) {
      if (source[j] === "{") depth++;
      else if (source[j] === "}") {
        depth--;
        if (depth === 0) {
          j++;
          break;
        }
      }
      j++;
    }
    i = j;
  }
  return out;
}

function parseDecls(blockBody) {
  const decls = new Map();
  for (const part of blockBody.split(";")) {
    const trimmed = part.trim();
    if (!trimmed) continue;
    const colon = trimmed.indexOf(":");
    if (colon < 0) continue;
    const name = trimmed.slice(0, colon).trim();
    const value = trimmed.slice(colon + 1).trim().replace(/\s+/g, " ");
    if (name.startsWith("--")) decls.set(name, value);
  }
  return decls;
}

function findBlock(source, predicate) {
  const re = /([^{]+)\{([^{}]*)\}/g;
  let match;
  while ((match = re.exec(source))) {
    const selector = match[1].trim().replace(/\s+/g, " ");
    if (predicate(selector)) return parseDecls(match[2]);
  }
  throw new Error(`no CSS block matching ${predicate}`);
}

const cleaned = stripMedia(stripComments(css));
const root = findBlock(cleaned, (s) => s === ":root");
const compact = findBlock(
  cleaned,
  (s) => s.includes('data-density="compact"') && !s.includes(":not("),
);

function resolveVar(value, decls, seen = new Set()) {
  const m = value.match(/^var\((--[\w-]+)\)$/);
  if (!m) return value;
  if (seen.has(m[1])) throw new Error(`circular var ${m[1]}`);
  seen.add(m[1]);
  const next = decls.get(m[1]);
  if (next == null) throw new Error(`unresolved ${m[1]}`);
  return resolveVar(next, decls, seen);
}

function parseRem(value) {
  const m = String(value).trim().match(/^(-?[\d.]+)rem$/i);
  if (!m) throw new Error(`expected rem, got ${value}`);
  return Number(m[1]);
}

function parseHex(value) {
  const m = String(value).trim().match(/^#([0-9a-fA-F]{6})$/);
  if (!m) throw new Error(`expected #rrggbb, got ${value}`);
  return parseInt(m[1], 16);
}

function parseUnitless(value) {
  const n = Number(String(value).trim());
  if (!Number.isFinite(n)) throw new Error(`expected number, got ${value}`);
  return n;
}

function parseWeight(value) {
  const n = Number(String(value).trim());
  if (!Number.isInteger(n) || n < 1 || n > 1000) {
    throw new Error(`expected font weight, got ${value}`);
  }
  return n;
}

/** color-mix(in srgb, var(--a) N%, var(--b)|transparent) */
function parseColorMix(value) {
  const m = value.match(
    /^color-mix\(\s*in\s+srgb\s*,\s*var\((--[\w-]+)\)\s+([\d.]+)%\s*,\s*(var\((--[\w-]+)\)|transparent)\s*\)$/i,
  );
  if (!m) throw new Error(`unsupported color-mix: ${value}`);
  return {
    from: m[1],
    percent: Number(m[2]),
    toTransparent: m[3] === "transparent",
    to: m[4] ?? null,
  };
}

function colorHex(name) {
  const raw = resolveVar(root.get(name), root);
  if (raw.startsWith("color-mix")) {
    throw new Error(`${name} is color-mix, not a hex base`);
  }
  return parseHex(raw);
}

const COLORS = [
  ["primary", "--color-primary"],
  ["background", "--color-background"],
  ["surface", "--color-surface"],
  ["foreground", "--color-foreground"],
  ["border", "--color-border"],
  ["active", "--color-active"],
  ["accent", "--color-accent"],
  ["secondary", "--color-secondary"],
  ["muted", "--color-muted"],
  ["muted_foreground", "--color-muted-foreground"],
  ["success", "--color-success"],
  ["info", "--color-info"],
  ["warning", "--color-warning"],
  ["error", "--color-error"],
  ["neutral", "--color-neutral"],
  ["text_on_primary", "--color-text-on-primary"],
];

const hexConsts = COLORS.map(([rust, cssName]) => ({
  rust,
  cssName,
  hex: colorHex(cssName),
}));

const hoverMix = parseColorMix(root.get("--color-hover"));
const textDisabledMix = parseColorMix(root.get("--color-text-disabled"));
if (
  hoverMix.toTransparent ||
  hoverMix.from !== "--color-neutral" ||
  hoverMix.to !== "--color-background"
) {
  throw new Error(`unexpected --color-hover mix: ${root.get("--color-hover")}`);
}
if (
  !textDisabledMix.toTransparent ||
  textDisabledMix.from !== "--color-muted-foreground"
) {
  throw new Error(
    `unexpected --color-text-disabled mix: ${root.get("--color-text-disabled")}`,
  );
}

const textPrimaryHex = parseHex(resolveVar(root.get("--color-text-primary"), root));
const textSecondaryHex = parseHex(
  resolveVar(root.get("--color-text-secondary"), root),
);
if (textPrimaryHex !== colorHex("--color-foreground")) {
  throw new Error("--color-text-primary must resolve to foreground for Theme::kai");
}
if (textSecondaryHex !== colorHex("--color-muted-foreground")) {
  throw new Error(
    "--color-text-secondary must resolve to muted-foreground for Theme::kai",
  );
}

const SPACE_KEYS = [
  ["space_1", "--space-1"],
  ["space_2", "--space-2"],
  ["space_3", "--space-3"],
  ["space_4", "--space-4"],
  ["page_gap", "--page-gap"],
  ["section_gap", "--section-gap"],
  ["card_padding", "--card-padding"],
];

const TYPE_SIZE_KEYS = [
  ["font_size_body", "--font-size-body"],
  ["font_size_sm", "--font-size-sm"],
  ["font_size_xs", "--font-size-xs"],
  ["font_size_page_title", "--font-size-page-title"],
  ["font_size_section_title", "--font-size-section-title"],
];

const CONTROL_KEYS = [
  ["height_md", "--control-height-md"],
  ["padding_y_md", "--control-padding-y-md"],
  ["height_compact", "--control-height-compact"],
  ["padding_y_compact", "--control-padding-y-compact"],
];

function densityTable(decls, label) {
  const rem = (cssName) => parseRem(decls.get(cssName));
  return {
    label,
    space: SPACE_KEYS.map(([rust, css]) => [rust, rem(css)]),
    type: TYPE_SIZE_KEYS.map(([rust, css]) => [rust, rem(css)]),
    control: CONTROL_KEYS.map(([rust, css]) => [rust, rem(css)]),
  };
}

for (const [, css] of [...SPACE_KEYS, ...TYPE_SIZE_KEYS, ...CONTROL_KEYS]) {
  if (!compact.has(css)) {
    throw new Error(`compact density missing ${css}`);
  }
}

const comfortable = densityTable(root, "comfortable");
const compactOnly = densityTable(compact, "compact");

const lineHeights = {
  tight: parseUnitless(root.get("--line-height-tight")),
  normal: parseUnitless(root.get("--line-height-normal")),
  relaxed: parseUnitless(root.get("--line-height-relaxed")),
};

const weights = {
  regular: parseWeight(root.get("--font-weight-regular")),
  medium: parseWeight(root.get("--font-weight-medium")),
  semibold: parseWeight(root.get("--font-weight-semibold")),
  bold: parseWeight(root.get("--font-weight-bold")),
};

function fmtRem(n) {
  if (Number.isInteger(n)) return `${n}.0`;
  const s = String(n);
  return s.includes(".") ? s : `${s}.0`;
}

function fmtHex(n) {
  return `0x${n.toString(16).padStart(6, "0")}`;
}

function emitDensityMod(table) {
  const lines = [`    pub mod ${table.label} {`];
  for (const [rust, rem] of table.space) {
    lines.push(`        pub const ${rust.toUpperCase()}: f32 = ${fmtRem(rem)};`);
  }
  for (const [rust, rem] of table.type) {
    lines.push(`        pub const ${rust.toUpperCase()}: f32 = ${fmtRem(rem)};`);
  }
  for (const [rust, rem] of table.control) {
    lines.push(`        pub const ${rust.toUpperCase()}: f32 = ${fmtRem(rem)};`);
  }
  lines.push(`    }`);
  return lines.join("\n");
}

const rust = `// @generated by scripts/gen-tokens.mjs from kai-ui-web/src/theme/tokens.css.
// Do not edit by hand.
// Regenerate: node kai-ui-desktop/scripts/gen-tokens.mjs
//
// Compile does not need kai-ui-web or node_modules; only regenerating does.
// Responsive @media density from tokens.css is intentionally omitted
// (see UNMAPPED_TOKENS in theme.rs).

use crate::color::Color;

use super::{ColorTokens, alpha};

/// Hex of \`--color-*\` bases (no color-mix).
${hexConsts
  .map((c) => `pub const ${c.rust.toUpperCase()}: u32 = ${fmtHex(c.hex)}; // ${c.cssName}`)
  .join("\n")}

/// \`color-mix(in srgb, var(--color-neutral) ${hoverMix.percent}%, var(--color-background))\`.
pub const HOVER_NEUTRAL_WEIGHT: f32 = ${fmtRem(hoverMix.percent / 100)};

/// \`color-mix(in srgb, var(--color-muted-foreground) ${textDisabledMix.percent}%, transparent)\`.
pub const TEXT_DISABLED_ALPHA: f32 = ${fmtRem(textDisabledMix.percent / 100)};

pub const LINE_HEIGHT_TIGHT: f32 = ${fmtRem(lineHeights.tight)};
pub const LINE_HEIGHT_NORMAL: f32 = ${fmtRem(lineHeights.normal)};
pub const LINE_HEIGHT_RELAXED: f32 = ${fmtRem(lineHeights.relaxed)};

pub const WEIGHT_REGULAR: u16 = ${weights.regular};
pub const WEIGHT_MEDIUM: u16 = ${weights.medium};
pub const WEIGHT_SEMIBOLD: u16 = ${weights.semibold};
pub const WEIGHT_BOLD: u16 = ${weights.bold};

/// Rem multipliers per density (\`1rem\` = [\`super::REM\`] at runtime).
pub mod density {
${emitDensityMod(comfortable)}

${emitDensityMod(compactOnly)}
}

/// Web color tokens for \`Theme::kai\`, including color-mix hover / text-disabled.
pub fn color_tokens() -> ColorTokens {
    let background = Color::hex(BACKGROUND);
    let neutral = Color::hex(NEUTRAL);
    let muted_foreground = Color::hex(MUTED_FOREGROUND);
    let foreground = Color::hex(FOREGROUND);
    ColorTokens {
        primary: Color::hex(PRIMARY),
        background,
        surface: Color::hex(SURFACE),
        foreground,
        border: Color::hex(BORDER),
        active: Color::hex(ACTIVE),
        accent: Color::hex(ACCENT),
        secondary: Color::hex(SECONDARY),
        muted: Color::hex(MUTED),
        muted_foreground,
        success: Color::hex(SUCCESS),
        info: Color::hex(INFO),
        warning: Color::hex(WARNING),
        error: Color::hex(ERROR),
        neutral,
        hover: neutral.mix(background, HOVER_NEUTRAL_WEIGHT),
        text_primary: foreground,
        text_secondary: muted_foreground,
        text_disabled: muted_foreground.with_alpha(alpha(TEXT_DISABLED_ALPHA)),
        text_on_primary: Color::hex(TEXT_ON_PRIMARY),
    }
}
`;

writeFileSync(outPath, rust);
console.log(`wrote ${outPath}`);
console.log(
  `${hexConsts.length} colors, comfortable+compact density, hover weight ${hoverMix.percent}%, text-disabled alpha ${textDisabledMix.percent}%`,
);
