"use client";
import React from "react";
import DotProgress from "../DotProgress/DotProgress";
import "./button.css";

export type ButtonVariant = "primary" | "secondary" | "outlined" | "outlinedSecondary" | "text" | "danger";
export type ButtonSize = "sm" | "md" | "lg";

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  children: React.ReactNode;
  variant?: ButtonVariant;
  size?: ButtonSize;
  className?: string;
  disabled?: boolean;
  loading?: boolean;
  /** Ignored (compat); use className w-full for full width */
  fullWidth?: boolean;
  /** Ignored (compat) */
  color?: string;
  "data-test-id"?: string;
}

const variantClasses: Record<string, string> = {
  primary: "fs-button fs-button--contained-primary",
  secondary: "fs-button fs-button--contained-secondary",
  outlined: "fs-button fs-button--outlined",
  outlinedSecondary: "fs-button fs-button--outlined-secondary",
  text: "fs-button fs-button--text",
  danger: "fs-button fs-button--contained-danger",
};

const sizeClasses: Record<string, string> = {
  sm: "px-3 py-1.5 text-sm",
  md: "px-4 py-2 fs-button--size-md",
  lg: "px-6 py-3 text-lg",
};

const disabledClasses: Record<string, string> = {
  primary: "fs-button fs-button--contained-primary opacity-50 cursor-not-allowed",
  secondary: "fs-button fs-button--contained-secondary opacity-50 cursor-not-allowed",
  outlined: "fs-button fs-button--outlined opacity-50 cursor-not-allowed",
  outlinedSecondary: "fs-button fs-button--outlined-secondary opacity-50 cursor-not-allowed",
  text: "fs-button fs-button--text opacity-50 cursor-not-allowed",
  danger: "fs-button fs-button--contained-danger opacity-50 cursor-not-allowed",
};

export const Button: React.FC<ButtonProps> = ({
  children,
  variant = "primary",
  size = "md",
  className = "",
  disabled = false,
  loading = false,
  ...props
}) => {
  const baseClasses = (disabled || loading)
    ? (disabledClasses[variant] || disabledClasses.primary)
    : (variantClasses[variant] || variantClasses.primary);

  const sizeClass = sizeClasses[size] || sizeClasses.md;
  const buttonClasses = `${baseClasses} ${sizeClass} ${className}`;

  // Filter out non-HTML button attributes
  const { color, fullWidth, ...buttonProps } = props;

  return (
    <button
      className={buttonClasses}
      data-test-id="button-root"
      disabled={disabled || loading}
      aria-busy={loading || undefined}
      {...buttonProps}
    >
      <div className="flex items-center justify-center gap-2 min-h-[1.25rem]">
        {loading ? (
          <DotProgress
            variant="wave"
            count={3}
            size={6}
            gap={3}
            colorPrimary="currentColor"
            colorNeutral="currentColor"
            className="fs-button__loading-dots"
          />
        ) : (
          children
        )}
      </div>
    </button>
  );
};
