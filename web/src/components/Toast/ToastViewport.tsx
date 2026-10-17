"use client";

import React from "react";

import "./toast.css";

export type ToastViewportProps = {
  children: React.ReactNode;
  className?: string;
};

const ToastViewport: React.FC<ToastViewportProps> = ({
  children,
  className = "",
}) => {
  if (!children || (Array.isArray(children) && children.length === 0)) {
    return null;
  }

  return (
    <div
      className={["fs-toast-viewport", className].filter(Boolean).join(" ")}
      aria-label="Notificaciones"
    >
      {children}
    </div>
  );
};

export default ToastViewport;
