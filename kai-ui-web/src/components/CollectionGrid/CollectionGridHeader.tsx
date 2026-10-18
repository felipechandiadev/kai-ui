"use client";

import React, { useCallback, useEffect, useRef, useState } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { Search } from "lucide-react";
import IconButton from "../IconButton";
import TextField from "../TextField";

export const COLLECTION_GRID_SEARCH_DEBOUNCE_MS = 300;

export type CollectionGridHeaderProps = {
  title?: string;
  subtitle?: string;
  addAction?: React.ReactNode;
  onAddClick?: () => void;
  addButtonAriaLabel?: string;
  showSearch?: boolean;
  searchParamName?: string;
  searchLabel?: string;
  searchPlaceholder?: string;
  onSearchChange?: (value: string) => void;
  "data-test-id"?: string;
};

/** Mismo TextField compacto que DataGrid Header (sin label flotante). */
function CollectionSearchField({
  name,
  value,
  onChange,
  placeholder,
  className = "",
  testId,
}: {
  name: string;
  value: string;
  onChange: (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => void;
  placeholder?: string;
  className?: string;
  testId: string;
}) {
  return (
    <TextField
      label=""
      name={name}
      value={value}
      onChange={onChange}
      placeholder={placeholder}
      density="compact"
      startAdornment={
        <Search className="h-4 w-4 shrink-0 text-secondary" aria-hidden />
      }
      className={className}
      aria-label="Buscar"
      data-test-id={testId}
    />
  );
}

export function CollectionGridHeader({
  title,
  subtitle,
  addAction,
  onAddClick,
  addButtonAriaLabel = "Añadir",
  showSearch = true,
  searchParamName = "search",
  searchPlaceholder = "Buscar...",
  onSearchChange,
  "data-test-id": dataTestId,
}: CollectionGridHeaderProps) {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const paramValue = searchParams.get(searchParamName) ?? "";
  const [searchInput, setSearchInput] = useState(paramValue);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    setSearchInput(paramValue);
  }, [paramValue]);

  const pushQuery = useCallback(
    (value: string) => {
      if (onSearchChange) {
        onSearchChange(value);
        return;
      }
      const next = new URLSearchParams(searchParams.toString());
      const trimmed = value.trim();
      if (trimmed) {
        next.set(searchParamName, trimmed);
      } else {
        next.delete(searchParamName);
      }
      const qs = next.toString();
      router.replace(qs ? `${pathname}?${qs}` : pathname, { scroll: false });
    },
    [onSearchChange, pathname, router, searchParamName, searchParams],
  );

  const handleSearchChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
  ) => {
    const v = e.target.value;
    setSearchInput(v);
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    debounceRef.current = setTimeout(() => {
      pushQuery(v);
      debounceRef.current = null;
    }, COLLECTION_GRID_SEARCH_DEBOUNCE_MS);
  };

  const titleTrim = title?.trim() ?? "";
  const subtitleTrim = subtitle?.trim() ?? "";
  const hasCustomAdd = addAction !== undefined;
  const hasDefaultAdd = !hasCustomAdd && onAddClick != null;
  const hasAddSlot = hasCustomAdd || hasDefaultAdd;
  const showTitleRow = hasAddSlot || Boolean(titleTrim) || showSearch;
  const showSubtitle = Boolean(subtitleTrim);
  const showHeaderBlock = showTitleRow || showSubtitle;

  if (!showHeaderBlock) {
    return null;
  }

  return (
    <header
      className="w-full min-w-0 shrink-0 pb-4"
      data-test-id={dataTestId ?? "collection-grid-header"}
    >
      {showTitleRow ? (
        <>
          {/* Misma idea que DataGrid: add/título a la izq., búsqueda a la der. */}
          <div className="flex w-full min-w-0 items-center gap-2 py-0">
            {hasAddSlot ? (
              <div
                className="flex shrink-0 items-center"
                data-test-id="collection-grid-add-wrap"
              >
                {hasCustomAdd ? (
                  addAction
                ) : hasDefaultAdd ? (
                  <IconButton
                    icon="Plus"
                    variant="action"
                    size="md"
                    ariaLabel={addButtonAriaLabel}
                    onClick={onAddClick!}
                    data-test-id="collection-grid-add"
                  />
                ) : null}
              </div>
            ) : null}
            {titleTrim ? (
              <div className="min-w-0 truncate text-lg font-semibold text-foreground">
                {titleTrim}
              </div>
            ) : null}
            <div className="min-w-0 flex-1" aria-hidden />
            {showSearch ? (
              <div className="hidden min-w-0 items-center justify-end gap-2 sm:flex">
                <CollectionSearchField
                  name={searchParamName}
                  value={searchInput}
                  onChange={handleSearchChange}
                  placeholder={searchPlaceholder}
                  className="w-full min-w-[10rem] max-w-[16rem]"
                  testId="collection-grid-search"
                />
              </div>
            ) : null}
          </div>
          {showSearch ? (
            <div className="mt-2 flex items-start justify-end gap-2 sm:hidden">
              <div className="flex min-w-0 max-w-xs flex-1 items-start">
                <CollectionSearchField
                  name={`${searchParamName}-mobile`}
                  value={searchInput}
                  onChange={handleSearchChange}
                  placeholder={searchPlaceholder}
                  className="w-full min-w-[10rem] max-w-[16rem]"
                  testId="collection-grid-search-mobile"
                />
              </div>
            </div>
          ) : null}
        </>
      ) : null}
      {showSubtitle ? (
        <p className="mt-1 text-sm text-muted-foreground">{subtitleTrim}</p>
      ) : null}
    </header>
  );
}
