import React from "react";

import {
  layoutPageContentClassName,
  layoutPageRootClassName,
  layoutPageTitleClassName,
} from "./layoutPageTokens";

export type EntityDetailLayoutProps = {
  /** Primary title (h1), e.g. entity name or code — name. */
  title?: React.ReactNode;
  /** Line under the title (document id, badges, etc.). */
  meta?: React.ReactNode;
  /** Optional control before the title (e.g. back IconButton). */
  backAction?: React.ReactNode;
  /** Actions aligned to the right of the title row (edit, delete, etc.). */
  headerActions?: React.ReactNode;
  /**
   * Section navigation under the identity header
   * (typically {@link EntityDetailSectionNav} from a Client Component).
   */
  sectionNav?: React.ReactNode;
  /** Active section panel content. */
  children: React.ReactNode;
  className?: string;
  contentClassName?: string;
  headerClassName?: string;
  /**
   * Prefix for the active panel wrapper id when `activeSectionId` is set
   * (`{idPrefix}-section-panel-{activeSectionId}`).
   */
  idPrefix?: string;
  /** When set, the content wrapper gets `role="tabpanel"` and matching aria ids. */
  activeSectionId?: string;
  "data-test-id"?: string;
};

/**
 * Mobile-first entity detail shell: identity header, section nav below, one panel.
 * Unlike {@link TabPageLayout}, section tabs sit under the header (not in the title row).
 *
 * Without `"use client"` — usable in Server and Client Components; pass a client
 * `sectionNav` when using controlled tabs + hash sync.
 */
export function EntityDetailLayout({
  title,
  meta,
  backAction,
  headerActions,
  sectionNav,
  children,
  className = "",
  contentClassName = "",
  headerClassName = "",
  idPrefix = "entity",
  activeSectionId,
  "data-test-id": dataTestId,
}: EntityDetailLayoutProps) {
  const showTitle = title != null && title !== false;
  const showMeta = meta != null && meta !== false;
  const showBack = backAction != null && backAction !== false;
  const showActions = headerActions != null && headerActions !== false;
  const showHeader = showTitle || showMeta || showBack || showActions;

  const panelId = activeSectionId
    ? `${idPrefix}-section-panel-${activeSectionId}`
    : undefined;
  const labelledBy = activeSectionId
    ? `${idPrefix}-section-tab-${activeSectionId}`
    : undefined;

  return (
    <div
      className={`${layoutPageRootClassName} ${className}`.trim()}
      data-test-id={dataTestId ?? "entity-detail-layout"}
    >
      {showHeader ? (
        <header
          className={`w-full min-w-0 border-b border-[var(--color-border,currentColor)] pb-4 ${headerClassName}`.trim()}
          data-test-id="entity-detail-layout-header"
        >
          <div
            className="flex min-w-0 flex-wrap items-start justify-between gap-3"
            data-test-id="entity-detail-layout-header-row"
          >
            <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-2">
              {showBack ? (
                <div data-test-id="entity-detail-layout-back">{backAction}</div>
              ) : null}
              {showTitle ? (
                <h1
                  className={`${layoutPageTitleClassName} min-w-0`}
                  data-test-id="entity-detail-layout-title"
                >
                  {title}
                </h1>
              ) : null}
            </div>
            {showActions ? (
              <div
                className="flex items-center gap-1"
                data-test-id="entity-detail-layout-header-actions"
              >
                {headerActions}
              </div>
            ) : null}
          </div>
          {showMeta ? (
            <div
              className="mt-3 text-sm text-[var(--color-muted-foreground,inherit)]"
              data-test-id="entity-detail-layout-meta"
            >
              {meta}
            </div>
          ) : null}
        </header>
      ) : null}

      {sectionNav ? (
        <div data-test-id="entity-detail-layout-section-nav">{sectionNav}</div>
      ) : null}

      <section
        id={panelId}
        role={activeSectionId ? "tabpanel" : undefined}
        aria-labelledby={labelledBy}
        className={`min-h-[16rem] ${layoutPageContentClassName} ${contentClassName}`.trim()}
        data-test-id="entity-detail-layout-content"
      >
        {children}
      </section>
    </div>
  );
}
