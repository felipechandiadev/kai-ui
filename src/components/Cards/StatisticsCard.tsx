import React from "react";
import { Card } from "./Card";

export type StatisticsValueTone = "primary" | "success" | "info" | "warning";

const valueToneClass: Record<StatisticsValueTone, string> = {
  primary: "fs-statistics-card__value--primary",
  success: "fs-statistics-card__value--success",
  info: "fs-statistics-card__value--info",
  warning: "fs-statistics-card__value--warning",
};

export interface StatisticsCardProps {
  label: string;
  value: React.ReactNode;
  hint?: React.ReactNode;
  tone?: StatisticsValueTone;
  compact?: boolean;
  className?: string;
  onClick?: () => void;
  "data-test-id"?: string;
}

export function StatisticsCard({
  label,
  value,
  hint,
  tone = "primary",
  compact = false,
  className = "",
  onClick,
  "data-test-id": dataTestId,
}: StatisticsCardProps) {
  const rootClass = [className, compact ? "fs-statistics-card--compact" : ""].filter(Boolean).join(" ");
  return (
    <Card className={rootClass} onClick={onClick} data-test-id={dataTestId}>
      <p className="fs-statistics-card__label">{label}</p>
      <p className={`fs-statistics-card__value ${valueToneClass[tone]}`}>{value}</p>
      {hint ? <p className="fs-statistics-card__hint">{hint}</p> : null}
    </Card>
  );
}
