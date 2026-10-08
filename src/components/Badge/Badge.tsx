"use client";
import React from 'react';
import './badge.css';

export type BadgeVariant = 'primary' | 'secondary' | 'success' | 'error' | 'warning' | 'info' | 'primary-outlined' | 'secondary-outlined' | 'success-outlined' | 'error-outlined' | 'warning-outlined' | 'info-outlined';

interface BadgeProps {
  children: React.ReactNode;
  variant?: BadgeVariant;
  className?: string;
}

const variantClass: Record<BadgeVariant, string> = {
  primary: 'fs-badge--primary',
  secondary: 'fs-badge--secondary',
  success: 'fs-badge--success',
  error: 'fs-badge--error',
  warning: 'fs-badge--warning',
  info: 'fs-badge--info',
  'primary-outlined': 'fs-badge--primary-outlined',
  'secondary-outlined': 'fs-badge--secondary-outlined',
  'success-outlined': 'fs-badge--success-outlined',
  'error-outlined': 'fs-badge--error-outlined',
  'warning-outlined': 'fs-badge--warning-outlined',
  'info-outlined': 'fs-badge--info-outlined',
};

const Badge: React.FC<BadgeProps> = ({ children, variant = 'primary', className = '' }) => {
  return (
    <span className={`fs-badge ${variantClass[variant]} ${className}`.trim()}>
      {children}
    </span>
  );
};

export default Badge;
