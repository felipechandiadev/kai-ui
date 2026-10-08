import React from "react";

import {
  layoutPageHeaderClassName,
  layoutPageSubtitleClassName,
  layoutPageTitleClassName,
} from "./layoutPageTokens";

export type PageLayoutHeaderProps = {
  title?: React.ReactNode;
  subtitle?: React.ReactNode;
  /** Slot derecho en la misma fila que el título (botones, tabs, toggles, etc.). */
  headerActions?: React.ReactNode;
  headerClassName?: string;
  titlesTestId?: string;
  subtitleTestId?: string;
  headerTestId?: string;
  headerRowTestId?: string;
  headerActionsTestId?: string;
};

function hasChunk(node: React.ReactNode): boolean {
  if (node == null || node === false) return false;
  if (typeof node === "string") return node.trim().length > 0;
  if (typeof node === "number") return true;
  return true;
}

/**
 * Encabezado compartido entre {@link BasicPageLayout} y {@link TabPageLayout}:
 * título y acciones en la misma fila; subtítulo debajo del título.
 */
export function PageLayoutHeader({
  title,
  subtitle,
  headerActions,
  headerClassName = "",
  titlesTestId = "page-layout-titles",
  subtitleTestId = "page-layout-subtitle",
  headerTestId = "page-layout-header",
  headerRowTestId = "page-layout-header-row",
  headerActionsTestId = "page-layout-header-actions",
}: PageLayoutHeaderProps) {
  const showTitle = hasChunk(title);
  const showSubtitle = hasChunk(subtitle);
  const showHeading = showTitle || showSubtitle;
  const showActions = hasChunk(headerActions);

  if (!showHeading && !showActions) {
    return null;
  }

  const titleRow =
    showTitle || showActions ? (
      <div
        className="flex w-full min-w-0 items-center gap-3"
        data-test-id={headerRowTestId}
      >
        {showTitle ? (
          <h1 className={`${layoutPageTitleClassName} min-w-0 flex-1 truncate`}>
            {title}
          </h1>
        ) : (
          <div className="min-w-0 flex-1" aria-hidden />
        )}
        {showActions ? (
          <div
            className="ml-auto flex min-w-0 max-w-full flex-1 flex-wrap items-center justify-end gap-2 pb-px"
            data-test-id={headerActionsTestId}
          >
            {headerActions}
          </div>
        ) : null}
      </div>
    ) : null;

  return (
    <header
      className={`${layoutPageHeaderClassName} ${headerClassName}`.trim()}
      data-test-id={headerTestId}
    >
      {showHeading && showActions ? (
        <div className="flex w-full min-w-0 flex-col gap-1" data-test-id={titlesTestId}>
          {titleRow}
          {showSubtitle ? (
            <p className={layoutPageSubtitleClassName} data-test-id={subtitleTestId}>
              {subtitle}
            </p>
          ) : null}
        </div>
      ) : showHeading ? (
        <div className="min-w-0" data-test-id={titlesTestId}>
          {showTitle ? (
            <h1 className={layoutPageTitleClassName}>{title}</h1>
          ) : null}
          {showSubtitle ? (
            <p className={layoutPageSubtitleClassName} data-test-id={subtitleTestId}>
              {subtitle}
            </p>
          ) : null}
        </div>
      ) : (
        <div
          className="flex min-w-0 w-full flex-wrap justify-start pb-px"
          data-test-id={headerActionsTestId}
        >
          {headerActions}
        </div>
      )}
    </header>
  );
}
