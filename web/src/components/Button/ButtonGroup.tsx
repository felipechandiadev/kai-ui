"use client";

import React, { createContext, useContext } from "react";
import { Button, type ButtonSize, type ButtonVariant } from "./Button";

type ButtonGroupContextValue = {
  size: ButtonSize;
  variant: ButtonVariant;
  density: ButtonGroupDensity;
};

export type ButtonGroupDensity = "default" | "compact";

const ButtonGroupContext = createContext<ButtonGroupContextValue | null>(null);

export type ButtonGroupProps = {
  children: React.ReactNode;
  className?: string;
  /** Variante por defecto de los ítems hijos. */
  variant?: ButtonVariant;
  size?: ButtonSize;
  /** `compact`: ~22px alto, texto 10px, padding horizontal mínimo. */
  density?: ButtonGroupDensity;
  "data-test-id"?: string;
  /** Etiqueta accesible del grupo (p. ej. «Modo de vista»). */
  "aria-label"?: string;
};

export type ButtonGroupItemProps = Omit<
  React.ComponentProps<typeof Button>,
  "fullWidth" | "color"
> & {
  /** Estado activo en grupos tipo toggle (segmented control). */
  selected?: boolean;
};

export function ButtonGroup({
  children,
  className = "",
  variant = "outlined",
  size,
  density = "default",
  "data-test-id": dataTestId = "button-group",
  "aria-label": ariaLabel,
}: ButtonGroupProps) {
  const resolvedSize = size ?? (density === "compact" ? "sm" : "md");

  return (
    <ButtonGroupContext.Provider value={{ size: resolvedSize, variant, density }}>
      <div
        role="group"
        aria-label={ariaLabel}
        className={`fs-button-group${density === "compact" ? " fs-button-group--compact" : ""} ${className}`.trim()}
        data-test-id={dataTestId}
      >
        {children}
      </div>
    </ButtonGroupContext.Provider>
  );
}

export function ButtonGroupItem({
  selected = false,
  variant,
  size,
  className = "",
  ...props
}: ButtonGroupItemProps) {
  const ctx = useContext(ButtonGroupContext);
  const resolvedVariant = variant ?? ctx?.variant ?? "outlined";
  const resolvedSize = size ?? ctx?.size ?? "md";

  return (
    <Button
      {...props}
      variant={selected ? "primary" : resolvedVariant}
      size={resolvedSize}
      aria-pressed={selected ? true : undefined}
      className={`fs-button-group__item${selected ? " fs-button-group__item--selected" : ""} ${className}`.trim()}
      data-test-id={props["data-test-id"] ?? "button-group-item"}
    />
  );
}

export type ButtonGroupToggleOption = {
  id: string;
  label: React.ReactNode;
  disabled?: boolean;
};

export type ButtonGroupToggleProps = {
  value: string;
  onChange: (id: string) => void;
  options: ButtonGroupToggleOption[];
  className?: string;
  variant?: ButtonVariant;
  size?: ButtonSize;
  density?: ButtonGroupDensity;
  "data-test-id"?: string;
  "aria-label"?: string;
};

/** Grupo de selección única (toggle) sobre `ButtonGroupItem`. */
export function ButtonGroupToggle({
  value,
  onChange,
  options,
  className,
  variant = "outlined",
  size,
  density = "default",
  "data-test-id": dataTestId = "button-group-toggle",
  "aria-label": ariaLabel,
}: ButtonGroupToggleProps) {
  return (
    <ButtonGroup
      className={className}
      variant={variant}
      size={size}
      density={density}
      data-test-id={dataTestId}
      aria-label={ariaLabel}
    >
      {options.map((opt) => (
        <ButtonGroupItem
          key={opt.id}
          type="button"
          selected={value === opt.id}
          disabled={opt.disabled}
          onClick={() => onChange(opt.id)}
          data-test-id={`${dataTestId}-${opt.id}`}
        >
          {opt.label}
        </ButtonGroupItem>
      ))}
    </ButtonGroup>
  );
}
