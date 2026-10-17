"use client";

import React, { createContext, useCallback, useMemo } from "react";
import type {
  NavRouter,
  NavSearchParams,
  NavSearchParamsInput,
  SetNavSearchParams,
} from "./types";

export type NavContextValue = {
  router: NavRouter;
  pathname: string;
  searchParams: NavSearchParams;
  setSearchParams: SetNavSearchParams;
};

export const NavContext = createContext<NavContextValue | null>(null);

function resolveSearchParamsInput(
  input: NavSearchParamsInput,
): URLSearchParams {
  if (input instanceof URLSearchParams) {
    return new URLSearchParams(input.toString());
  }
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(input)) {
    if (value === undefined) continue;
    if (Array.isArray(value)) {
      for (const item of value) {
        params.append(key, item);
      }
    } else {
      params.set(key, value);
    }
  }
  return params;
}

export type NavProviderProps = {
  router: NavRouter;
  pathname: string;
  searchParams: NavSearchParams;
  children: React.ReactNode;
  /** When omitted, `setSearchParams` builds `pathname?query` and calls `router.replace`. */
  setSearchParams?: SetNavSearchParams;
};

export function NavProvider({
  router,
  pathname,
  searchParams,
  children,
  setSearchParams: setSearchParamsProp,
}: NavProviderProps) {
  const setSearchParams = useCallback<SetNavSearchParams>(
    (next, options) => {
      if (setSearchParamsProp) {
        setSearchParamsProp(next, options);
        return;
      }
      const resolved =
        typeof next === "function"
          ? resolveSearchParamsInput(next(searchParams))
          : resolveSearchParamsInput(next);
      const qs = resolved.toString();
      router.replace(qs ? `${pathname}?${qs}` : pathname, options);
    },
    [pathname, router, searchParams, setSearchParamsProp],
  );

  const value = useMemo(
    () => ({
      router,
      pathname,
      searchParams,
      setSearchParams,
    }),
    [router, pathname, searchParams, setSearchParams],
  );

  return (
    <NavContext.Provider value={value}>{children}</NavContext.Provider>
  );
}
