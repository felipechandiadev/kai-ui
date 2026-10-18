/**
 * Breakpoints alineados a Tailwind default.
 * Paridad con media queries en `tokens.css`.
 */
export const KAI_BREAKPOINTS = {
  xs: 0,
  sm: 640,
  md: 768,
  lg: 1024,
  xl: 1280,
  "2xl": 1536,
} as const;

export type KaiBreakpoint = keyof typeof KAI_BREAKPOINTS;

export const KAI_BREAKPOINT_ORDER: KaiBreakpoint[] = [
  "xs",
  "sm",
  "md",
  "lg",
  "xl",
  "2xl",
];

export function resolveKaiBreakpoint(width: number): KaiBreakpoint {
  if (width < KAI_BREAKPOINTS.sm) return "xs";
  if (width < KAI_BREAKPOINTS.md) return "sm";
  if (width < KAI_BREAKPOINTS.lg) return "md";
  if (width < KAI_BREAKPOINTS.xl) return "lg";
  if (width < KAI_BREAKPOINTS["2xl"]) return "xl";
  return "2xl";
}

export function kaiBreakpointAtLeast(
  current: KaiBreakpoint,
  minimum: KaiBreakpoint,
): boolean {
  return (
    KAI_BREAKPOINT_ORDER.indexOf(current) >= KAI_BREAKPOINT_ORDER.indexOf(minimum)
  );
}
