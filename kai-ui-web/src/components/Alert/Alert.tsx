"use client";
import React from "react";

import "./alert.css";

export type AlertVariant = "success" | "info" | "warning" | "error";

interface AlertProps {
  variant?: AlertVariant;
  children: React.ReactNode;
  className?: string;
  ["data-test-id"]?: string;
}

const variantClass: Record<AlertVariant, string> = {
  success: "fs-alert--success",
  info: "fs-alert--info",
  warning: "fs-alert--warning",
  error: "fs-alert--error",
};

const Alert: React.FC<AlertProps> = ({ variant = "info", children, className = "", ...props }) => {
  const dataTestId = props["data-test-id"];
  return (
    <div className="relative w-full">
      <div className="fs-alert__scrim" aria-hidden />
      <div
        className={`fs-alert ${variantClass[variant]} ${className}`.trim()}
        role="alert"
        data-test-id={dataTestId || `alert-${variant}`}
      >
        {children}
      </div>
    </div>
  );
};

export default Alert;
