import type { HTMLAttributes } from "react";
import "./skeleton.css";

export type SkeletonProps = HTMLAttributes<HTMLDivElement>;

export function Skeleton({ className = "", ...props }: SkeletonProps) {
  return <div className={`fs-skeleton ${className}`.trim()} aria-hidden {...props} />;
}

export default Skeleton;
