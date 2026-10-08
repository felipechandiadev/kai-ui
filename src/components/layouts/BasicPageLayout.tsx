import React from "react";

import { PageLayoutHeader } from "./PageLayoutHeader";
import {
  layoutPageContentClassName,
  layoutPageRootClassName,
} from "./layoutPageTokens";

export type BasicPageLayoutProps = {
  /** Título principal (h1). */
  title?: React.ReactNode;
  /** Texto o nodo bajo el título. */
  subtitle?: React.ReactNode;
  /**
   * Botones, toggles u otros controles en la misma fila que el título (`md+`),
   * alineados a la derecha — mismo layout visual que `tabs` en {@link TabPageLayout}.
   */
  headerActions?: React.ReactNode;
  /**
   * @deprecated Usa `headerActions`.
   */
  headerEnd?: React.ReactNode;
  /** Contenido principal de la página. */
  children: React.ReactNode;
  className?: string;
  /** Clases extra en el contenedor del cuerpo (debajo del encabezado). */
  contentClassName?: string;
  /** Clases extra en el contenedor del encabezado. */
  headerClassName?: string;
  "data-test-id"?: string;
};

/**
 * Layout mínimo de página: encabezado opcional (título + subtítulo + acciones) y área de contenido.
 * Sin `"use client"` — usable en Server y Client Components.
 */
export function BasicPageLayout({
  title,
  subtitle,
  headerActions,
  headerEnd,
  children,
  className = "",
  contentClassName = "",
  headerClassName = "",
  "data-test-id": dataTestId,
}: BasicPageLayoutProps) {
  const actions = headerActions ?? headerEnd;

  return (
    <div
      className={`${layoutPageRootClassName} ${className}`.trim()}
      data-test-id={dataTestId ?? "basic-page-layout"}
    >
      <PageLayoutHeader
        title={title}
        subtitle={subtitle}
        headerActions={actions}
        headerClassName={headerClassName}
        titlesTestId="basic-page-layout-titles"
        subtitleTestId="basic-page-layout-subtitle"
        headerTestId="basic-page-layout-header"
        headerRowTestId="basic-page-layout-header-row"
        headerActionsTestId="basic-page-layout-header-actions"
      />

      <section
        className={`${layoutPageContentClassName} ${contentClassName}`.trim()}
        data-test-id="basic-page-layout-content"
      >
        {children}
      </section>
    </div>
  );
}
