"use client";

import { useEffect, useState } from "react";
import {
  kaiBreakpointAtLeast,
  resolveKaiBreakpoint,
  type KaiBreakpoint,
} from "../theme/breakpoints";

export type UseBreakpointResult = {
  width: number;
  breakpoint: KaiBreakpoint;
  isXs: boolean;
  isSm: boolean;
  isMd: boolean;
  isLg: boolean;
  isXl: boolean;
  is2xl: boolean;
  atLeast: (bp: KaiBreakpoint) => boolean;
  isMobile: boolean;
  isTablet: boolean;
  isDesktop: boolean;
};

const SSR_WIDTH = 1024;

function fromWidth(width: number): UseBreakpointResult {
  const breakpoint = resolveKaiBreakpoint(width);
  return {
    width,
    breakpoint,
    isXs: breakpoint === "xs",
    isSm: breakpoint === "sm",
    isMd: breakpoint === "md",
    isLg: breakpoint === "lg",
    isXl: breakpoint === "xl",
    is2xl: breakpoint === "2xl",
    atLeast: (bp) => kaiBreakpointAtLeast(breakpoint, bp),
    isMobile: width < 640,
    isTablet: width >= 640 && width < 1024,
    isDesktop: width >= 1024,
  };
}

/**
 * Identifica el breakpoint del viewport (Tailwind-aligned).
 * Tipografía/spacing: preferir tokens CSS en `tokens.css`.
 * Este hook: layout estructural (drawer, columnas, etc.).
 */
export function useBreakpoint(): UseBreakpointResult {
  const [state, setState] = useState<UseBreakpointResult>(() => fromWidth(SSR_WIDTH));

  useEffect(() => {
    if (typeof window === "undefined") return;
    const update = () => setState(fromWidth(window.innerWidth));
    update();
    window.addEventListener("resize", update);
    return () => window.removeEventListener("resize", update);
  }, []);

  return state;
}
