"use client";

import DotProgress from "../DotProgress/DotProgress";

export type LoadingStateProps = {
  className?: string;
  size?: number;
  label?: string;
};

export default function LoadingState({
  className = "flex items-center justify-center py-4",
  size,
  label = "Cargando",
}: LoadingStateProps) {
  return (
    <div className={className} role="status" aria-label={label}>
      <DotProgress size={size} />
    </div>
  );
}
