import type { CSSProperties } from 'react';

export type AppearanceColors = {
  primary: string;
  secondary: string;
  background: string;
  foreground: string;
  accent: string;
};

export const DEFAULT_APPEARANCE: AppearanceColors = {
  primary: '#1e73ae',
  secondary: '#00deef',
  background: '#ffffff',
  foreground: '#131615',
  accent: '#02578b',
};

export const APPEARANCE_COLOR_KEYS = [
  'primary',
  'secondary',
  'background',
  'foreground',
  'accent',
] as const;

const HEX = /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/;

export function normalizeHexColor(value: string): string | null {
  const trimmed = value.trim();
  if (!HEX.test(trimmed)) return null;
  const body = trimmed.slice(1);
  const full = body.length === 3 ? body.split('').map((char) => char + char).join('') : body;
  return `#${full.toLowerCase()}`;
}

function hexToRgb(hex: string): [number, number, number] {
  return [
    parseInt(hex.slice(1, 3), 16),
    parseInt(hex.slice(3, 5), 16),
    parseInt(hex.slice(5, 7), 16),
  ];
}

function rgbToHex(r: number, g: number, b: number): string {
  const clamp = (channel: number) => Math.max(0, Math.min(255, Math.round(channel)));
  return `#${[r, g, b].map((channel) => clamp(channel).toString(16).padStart(2, '0')).join('')}`;
}

function mix(base: string, other: string, weight: number): string {
  const [br, bg, bb] = hexToRgb(base);
  const [or, og, ob] = hexToRgb(other);
  return rgbToHex(
    br * weight + or * (1 - weight),
    bg * weight + og * (1 - weight),
    bb * weight + ob * (1 - weight),
  );
}

function darken(hex: string, amount: number): string {
  return mix(hex, '#000000', 1 - amount);
}

function relativeLuminance(hex: string): number {
  const [r, g, b] = hexToRgb(hex).map((channel) => {
    const scaled = channel / 255;
    return scaled <= 0.03928 ? scaled / 12.92 : ((scaled + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function textOnColor(hex: string): string {
  return relativeLuminance(hex) > 0.55 ? '#131615' : '#ffffff';
}

export function resolveAppearance(
  input?: Partial<AppearanceColors> | null,
): AppearanceColors {
  const pick = (key: keyof AppearanceColors) => {
    const raw = input?.[key];
    if (typeof raw !== 'string') return DEFAULT_APPEARANCE[key];
    return normalizeHexColor(raw) ?? DEFAULT_APPEARANCE[key];
  };
  return {
    primary: pick('primary'),
    secondary: pick('secondary'),
    background: pick('background'),
    foreground: pick('foreground'),
    accent: pick('accent'),
  };
}

export type AppearanceCssVars = Record<`--color-${string}`, string>;

function channels(hex: string): string {
  const [r, g, b] = hexToRgb(hex);
  return `${r} ${g} ${b}`;
}

export function buildAppearanceCssVars(colors: AppearanceColors): AppearanceCssVars {
  const active = darken(colors.primary, 0.18);
  const border = mix(colors.secondary, colors.background, 0.35);
  const muted = mix(colors.secondary, colors.background, 0.22);
  const mutedForeground = mix(colors.foreground, colors.background, 0.55);
  const surface = mix(colors.background, colors.secondary, 0.94);
  const vars: AppearanceCssVars = {};

  const set = (name: string, hex: string) => {
    vars[`--color-${name}`] = hex;
    vars[`--color-${name}-rgb`] = channels(hex);
  };

  set('primary', colors.primary);
  set('secondary', colors.secondary);
  set('background', colors.background);
  set('foreground', colors.foreground);
  set('accent', colors.accent);
  set('active', active);
  set('info', colors.primary);
  set('border', border);
  set('muted', muted);
  set('muted-foreground', mutedForeground);
  set('surface', surface);
  vars['--color-link'] = colors.primary;
  vars['--color-text-primary'] = colors.foreground;
  vars['--color-text-link'] = colors.primary;
  vars['--color-text-on-primary'] = textOnColor(colors.primary);
  return vars;
}

export function appearanceStyle(colors: AppearanceColors): CSSProperties {
  return buildAppearanceCssVars(colors) as CSSProperties;
}
