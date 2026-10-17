import React from "react";

import { PageLayoutHeader } from "./PageLayoutHeader";
import {
  layoutPageContentClassName,
  layoutPageRootClassName,
  layoutPageRootClassNameCompact,
} from "./layoutPageTokens";

export type TabPageLayoutProps = {
  /** Título principal (h1), en el bloque de títulos (izquierda). */
  title?: React.ReactNode;
  /** Texto o nodo bajo el título, en el mismo bloque. */
  subtitle?: React.ReactNode;
  /**
   * Navegación por pestañas (p. ej. `<Tabs items={...} />` desde un Client Component).
   * Misma fila que el bloque de títulos, alineada a la derecha.
   */
  tabs?: React.ReactNode;
  children: React.ReactNode;
  className?: string;
  contentClassName?: string;
  /** Clases extra en el contenedor del encabezado. */
  headerClassName?: string;
  /** Menos espacio vertical entre la fila de pestañas y el contenido. */
  compact?: boolean;
  "data-test-id"?: string;
};

/**
 * Layout de página con pestañas: comparte encabezado con {@link BasicPageLayout}
 * (raíz, título `text-lg`, subtítulo, bloque de contenido).
 *
 * Sin `"use client"` — usable en Server y Client Components (el slot `tabs` puede ser un Client Component).
 */
export function TabPageLayout({
  title,
  subtitle,
  tabs,
  children,
  className = "",
  contentClassName = "",
  headerClassName = "",
  compact = false,
  "data-test-id": dataTestId,
}: TabPageLayoutProps) {
  const rootBase = compact ? layoutPageRootClassNameCompact : layoutPageRootClassName;

  return (
    <div
      className={`${rootBase} ${className}`.trim()}
      data-test-id={dataTestId ?? "tab-page-layout"}
    >
      <PageLayoutHeader
        title={title}
        subtitle={subtitle}
        headerActions={tabs}
        headerClassName={headerClassName}
        titlesTestId="tab-page-layout-titles"
        subtitleTestId="tab-page-layout-subtitle"
        headerTestId="tab-page-layout-header"
        headerRowTestId="tab-page-layout-header-row"
        headerActionsTestId="tab-page-layout-tabs"
      />

      <section
        className={`${layoutPageContentClassName} ${contentClassName}`.trim()}
        data-test-id="tab-page-layout-content"
      >
        {children}
      </section>
    </div>
  );
}
