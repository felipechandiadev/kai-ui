"use client";
import React from "react";
import "./button.css";

interface ButtonPillProps {
  children: React.ReactNode;
  variant?: "primary" | "secondary" | "outlined";
  className?: string;
  disabled?: boolean;
  [key: string]: any;
}

const pillVariantClasses: Record<string, string> = {
  primary: "fs-pill fs-pill--primary",
  secondary: "fs-pill fs-pill--secondary",
  outlined: "fs-pill fs-pill--outlined",
};

const pillDisabledClasses: Record<string, string> = {
  primary: "fs-pill fs-pill--primary opacity-50 cursor-not-allowed",
  secondary: "fs-pill fs-pill--secondary opacity-50 cursor-not-allowed",
  outlined: "fs-pill fs-pill--outlined opacity-50 cursor-not-allowed",
};

export const ButtonPill: React.FC<ButtonPillProps> = ({
  children,
  variant = "primary",
  className = "",
  disabled = false,
  ...props
}) => {
  const buttonClasses = disabled
    ? `${pillDisabledClasses[variant] || pillDisabledClasses.primary} ${className}`
    : `${pillVariantClasses[variant] || pillVariantClasses.primary} ${className}`;

  return (
    <button
      className={buttonClasses}
      data-test-id="button-pill-root"
      disabled={disabled}
      {...props}
    >
      {children}
    </button>
  );
};
