"use client";

import React, { Suspense, useMemo } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { NavProvider } from "./context";
import type { NavRouter } from "./types";

function useNextRouter(): NavRouter {
  const nextRouter = useRouter();

  return useMemo<NavRouter>(
    () => ({
      push(href, options) {
        nextRouter.push(href, options);
      },
      replace(href, options) {
        nextRouter.replace(href, options);
      },
      back() {
        nextRouter.back();
      },
    }),
    [nextRouter],
  );
}

function NextNavProviderInner({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const router = useNextRouter();

  const navSearchParams = useMemo(
    () => new URLSearchParams(searchParams.toString()),
    [searchParams],
  );

  return (
    <NavProvider
      router={router}
      pathname={pathname}
      searchParams={navSearchParams}
    >
      {children}
    </NavProvider>
  );
}

function NextNavFromSearch({
  children,
  search,
}: {
  children: React.ReactNode;
  search: string;
}) {
  const pathname = usePathname();
  const router = useNextRouter();
  const navSearchParams = useMemo(() => new URLSearchParams(search), [search]);

  return (
    <NavProvider router={router} pathname={pathname} searchParams={navSearchParams}>
      {children}
    </NavProvider>
  );
}

/** Bridges Next.js App Router into the navigation context. */
export function NextNavProvider({
  children,
  search,
}: {
  children: React.ReactNode;
  /**
   * Query string without `?`, passed from a Server Component.
   * Skips `useSearchParams`, which suspends and would render DataGrid outside this provider.
   */
  search?: string;
}) {
  if (search !== undefined) {
    return <NextNavFromSearch search={search}>{children}</NextNavFromSearch>;
  }

  return (
    <Suspense fallback={children}>
      <NextNavProviderInner>{children}</NextNavProviderInner>
    </Suspense>
  );
}
