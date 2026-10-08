"use client";
import React from "react";
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
      {...buttonProps}
    >
      <div className="flex items-center justify-center gap-2">
        {loading && (
          <svg className="animate-spin h-4 w-4 text-current" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
            <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
            <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
        )}
        {children}
      </div>
    </button>
  );
};
