"use client";

import React, { Suspense } from "react";
import DotProgress from "../DotProgress/DotProgress";
import CollectionGridView, { type CollectionGridProps } from "./CollectionGrid";

function CollectionGridFallback({ title }: { title?: string }) {
  return (
    <div
      className="flex min-h-[16rem] flex-col gap-3 rounded-md border border-border p-4"
      data-test-id="collection-grid-loading"
    >
      <div className="flex items-center gap-2">
        <div className="h-10 w-10 shrink-0 animate-pulse rounded-md bg-neutral/60" aria-hidden />
        {title?.trim() ? (
          <div className="h-7 w-40 animate-pulse rounded-md bg-neutral/60" aria-hidden />
        ) : null}
      </div>
      <div className="flex flex-1 items-center justify-center">
        <DotProgress count={3} variant="wave" />
      </div>
    </div>
  );
}

const CollectionGrid: React.FC<CollectionGridProps> = (props) => {
  return (
    <Suspense fallback={<CollectionGridFallback title={props.title} />}>
      <CollectionGridView {...props} />
    </Suspense>
  );
};

export default CollectionGrid;
export type { CollectionGridProps };
