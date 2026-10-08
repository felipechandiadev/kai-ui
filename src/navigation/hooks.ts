"use client";

import { useContext } from "react";
import { NavContext } from "./context";
import type { NavRouter, NavSearchParams, SetNavSearchParams } from "./types";

const MISSING_PROVIDER =
  "@kai/ui navigation hooks require NavProvider (NextNavProvider or MemoryNavProvider).";

function useNavContext() {
  const ctx = useContext(NavContext);
  if (!ctx) {
    throw new Error(MISSING_PROVIDER);
  }
  return ctx;
}

export function useRouter(): NavRouter {
  return useNavContext().router;
}

export function usePathname(): string {
  return useNavContext().pathname;
}

export function useSearchParams(): NavSearchParams {
  return useNavContext().searchParams;
}

export function useSetSearchParams(): SetNavSearchParams {
  return useNavContext().setSearchParams;
}
