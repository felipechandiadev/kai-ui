"use client";

import React from "react";
import {
  AlertTriangle,
  CheckCircle2,
  Info,
  X,
  XCircle,
  type LucideIcon,
} from "lucide-react";

import "./toast.css";

export type ToastVariant = "success" | "info" | "warning" | "error";

export type ToastProps = {
  variant?: ToastVariant;
  message: string;
  onClose?: () => void;
  /** Applied while the toast is leaving the stack. */
  exiting?: boolean;
  className?: string;
  ["data-test-id"]?: string;
  onMouseEnter?: React.MouseEventHandler<HTMLDivElement>;
  onMouseLeave?: React.MouseEventHandler<HTMLDivElement>;
};

const variantClass: Record<ToastVariant, string> = {
  success: "fs-toast--success",
  info: "fs-toast--info",
  warning: "fs-toast--warning",
  error: "fs-toast--error",
};

const variantIcon: Record<ToastVariant, LucideIcon> = {
  success: CheckCircle2,
  info: Info,
  warning: AlertTriangle,
  error: XCircle,
};

const Toast: React.FC<ToastProps> = ({
  variant = "info",
  message,
  onClose,
  exiting = false,
  className = "",
  onMouseEnter,
  onMouseLeave,
  ...props
}) => {
  const dataTestId = props["data-test-id"];
  const Icon = variantIcon[variant];

  return (
    <div
      className={[
        "fs-toast",
        variantClass[variant],
        exiting ? "fs-toast--exiting" : "fs-toast--entering",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
      role="status"
      aria-live="polite"
      data-test-id={dataTestId || `toast-${variant}`}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      <Icon className="fs-toast__icon" aria-hidden size={20} strokeWidth={2} />
      <div className="fs-toast__body">{message}</div>
      {onClose ? (
        <button
          type="button"
          className="fs-toast__close"
          aria-label="Cerrar notificación"
          onClick={onClose}
        >
          <X size={16} strokeWidth={2} aria-hidden />
        </button>
      ) : null}
    </div>
  );
};

export default Toast;
