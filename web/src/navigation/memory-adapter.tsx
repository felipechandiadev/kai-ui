"use client";

import React, { useMemo, useSyncExternalStore } from "react";
import { NavProvider } from "./context";
import type { NavRouter } from "./types";

export type MemoryNavAdapter = {
  router: NavRouter;
  subscribe: (onStoreChange: () => void) => () => void;
  getPathname: () => string;
  getSearch: () => string;
};

function getDefaultPathname(): string {
  if (typeof window === "undefined") return "/";
  return window.location.pathname;
}

function getDefaultSearch(): string {
  if (typeof window === "undefined") return "";
  return window.location.search.startsWith("?")
    ? window.location.search.slice(1)
    : window.location.search;
}

function notifyNavigation() {
  if (typeof window !== "undefined") {
    window.dispatchEvent(new PopStateEvent("popstate"));
  }
}

/** Browser history adapter for Vite apps and tests (no Next.js). */
export function createMemoryNavAdapter(
  initialPathname = getDefaultPathname(),
): MemoryNavAdapter {
  let pathnameSnapshot =
    typeof window !== "undefined" ? window.location.pathname : initialPathname;

  const router: NavRouter = {
    push(href) {
      if (typeof window === "undefined") return;
      window.history.pushState({}, "", href);
      pathnameSnapshot = window.location.pathname;
      notifyNavigation();
    },
    replace(href) {
      if (typeof window === "undefined") return;
      window.history.replaceState({}, "", href);
      pathnameSnapshot = window.location.pathname;
      notifyNavigation();
    },
    back() {
      if (typeof window === "undefined") return;
      window.history.back();
    },
  };

  return {
    router,
    subscribe(onStoreChange) {
      if (typeof window === "undefined") {
        return () => {};
      }
      const handler = () => {
        pathnameSnapshot = window.location.pathname;
        onStoreChange();
      };
      window.addEventListener("popstate", handler);
      return () => window.removeEventListener("popstate", handler);
    },
    getPathname() {
      if (typeof window !== "undefined") {
        return window.location.pathname;
      }
      return pathnameSnapshot;
    },
    getSearch() {
      if (typeof window !== "undefined") {
        const raw = window.location.search;
        return raw.startsWith("?") ? raw.slice(1) : raw;
      }
      return "";
    },
  };
}

export type MemoryNavProviderProps = {
  children: React.ReactNode;
  adapter?: MemoryNavAdapter;
  /** Used for SSR snapshot when `window` is unavailable. */
  initialPathname?: string;
};

export function MemoryNavProvider({
  children,
  adapter: adapterProp,
  initialPathname = "/",
}: MemoryNavProviderProps) {
  const adapter = useMemo(
    () => adapterProp ?? createMemoryNavAdapter(initialPathname),
    [adapterProp, initialPathname],
  );

  const pathname = useSyncExternalStore(
    adapter.subscribe,
    adapter.getPathname,
    () => initialPathname,
  );

  const searchString = useSyncExternalStore(
    adapter.subscribe,
    adapter.getSearch,
    getDefaultSearch,
  );

  const searchParams = useMemo(
    () => new URLSearchParams(searchString),
    [searchString],
  );

  return (
    <NavProvider
      router={adapter.router}
      pathname={pathname}
      searchParams={searchParams}
    >
      {children}
    </NavProvider>
  );
}
