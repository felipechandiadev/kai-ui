export interface NavRouterOptions {
  scroll?: boolean;
}

export interface NavRouter {
  push(href: string, options?: NavRouterOptions): void;
  replace(href: string, options?: NavRouterOptions): void;
  back(): void;
}

/** Subset of URLSearchParams used by DataGrid / CollectionPageLayout. */
export interface NavSearchParams {
  get(name: string): string | null;
  has(name: string): boolean;
  set(name: string, value: string): void;
  delete(name: string): void;
  toString(): string;
  entries(): IterableIterator<[string, string]>;
}

export type NavSearchParamsInput =
  | URLSearchParams
  | Record<string, string | string[] | undefined>;

export type SetNavSearchParams = (
  next: NavSearchParamsInput | ((prev: NavSearchParams) => NavSearchParamsInput),
  options?: NavRouterOptions,
) => void;
