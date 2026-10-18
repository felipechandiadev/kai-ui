"use client";

import { useEffect } from "react";
import { useBreakpoint } from "../hooks/useBreakpoint";

export type KaiResponsiveRootProps = {
  children: React.ReactNode;
  /**
   * Escribe `data-kai-bp` en `<html>` (default true).
   * La densidad tipográfica/spacing la controlan las media queries de `tokens.css`.
   */
  syncHtmlAttribute?: boolean;
};

/**
 * Montar cerca de la raíz de la PWA (p. ej. Providers).
 * Expone el breakpoint en `document.documentElement.dataset.kaiBp`.
 */
export function KaiResponsiveRoot({
  children,
  syncHtmlAttribute = true,
}: KaiResponsiveRootProps) {
  const { breakpoint } = useBreakpoint();

  useEffect(() => {
    if (!syncHtmlAttribute || typeof document === "undefined") return;
    document.documentElement.dataset.kaiBp = breakpoint;
    return () => {
      delete document.documentElement.dataset.kaiBp;
    };
  }, [breakpoint, syncHtmlAttribute]);

  return <>{children}</>;
}
