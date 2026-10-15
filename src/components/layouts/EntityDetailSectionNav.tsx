"use client";

import React from "react";

export type EntityDetailSectionTab = {
  id: string;
  label: string;
};

export type EntityDetailSectionNavProps = {
  tabs: EntityDetailSectionTab[];
  activeId: string;
  onSelect: (id: string) => void;
  /** Accessible name for the tab list. */
  ariaLabel?: string;
  /**
   * Prefix for tab / panel ids (e.g. `staff` → `staff-section-tab-resumen`).
   * Defaults to `entity`.
   */
  idPrefix?: string;
  className?: string;
  "data-test-id"?: string;
};

/**
 * Underline section tabs for entity detail pages (mobile-friendly wrap).
 * Controlled: pair with a single tabpanel whose id is `{idPrefix}-section-panel-{id}`.
 */
export function EntityDetailSectionNav({
  tabs,
  activeId,
  onSelect,
  ariaLabel = "Secciones",
  idPrefix = "entity",
  className = "",
  "data-test-id": dataTestId,
}: EntityDetailSectionNavProps) {
  return (
    <nav
      className={`flex flex-wrap gap-1 border-b border-[var(--color-border,currentColor)] ${className}`.trim()}
      aria-label={ariaLabel}
      data-test-id={dataTestId ?? "entity-detail-section-nav"}
    >
      {tabs.map((tab) => {
        const isActive = tab.id === activeId;
        return (
          <button
            key={tab.id}
            type="button"
            role="tab"
            aria-selected={isActive}
            aria-controls={`${idPrefix}-section-panel-${tab.id}`}
            id={`${idPrefix}-section-tab-${tab.id}`}
            className={`cursor-pointer border-0 border-b-2 bg-transparent px-3 py-2 text-sm font-medium transition-colors ${
              isActive
                ? "border-[var(--color-primary,currentColor)] text-[var(--color-foreground,inherit)]"
                : "border-transparent text-[var(--color-muted-foreground,inherit)] hover:text-[var(--color-foreground,inherit)]"
            }`}
            onClick={() => onSelect(tab.id)}
            data-test-id={`${idPrefix}-detail-tab-${tab.id}`}
          >
            {tab.label}
          </button>
        );
      })}
    </nav>
  );
}
