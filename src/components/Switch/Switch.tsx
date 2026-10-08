'use client';

import React, { type CSSProperties, type KeyboardEvent } from 'react';

import './switch.css';

export type SwitchOptionLabels = {
  /** Opción cuando `checked` es false */
  off: string;
  /** Opción cuando `checked` es true */
  on: string;
};

export type SwitchDensity = 'default' | 'compact';

interface SwitchProps {
  checked?: boolean;
  onChange?: (checked: boolean) => void;
  /** Etiqueta única (izquierda o derecha del control) */
  label?: string;
  labelPosition?: 'left' | 'right';
  /** Dos etiquetas (off | switch | on); la activa resalta según `checked` */
  optionLabels?: SwitchOptionLabels;
  disabled?: boolean;
  /** `compact`: track más pequeño, sin margen superior (p. ej. dentro de TextField inline). */
  density?: SwitchDensity;
  className?: string;
  ['data-test-id']?: string;
}

const Switch: React.FC<SwitchProps> = ({
  checked = false,
  onChange,
  label,
  labelPosition = 'left',
  optionLabels,
  disabled = false,
  density = 'default',
  className = '',
  ...props
}) => {
  const isCompact = density === 'compact';

  const setChecked = (next: boolean) => {
    if (disabled || next === checked) {
      return;
    }
    onChange?.(next);
  };

  const toggle = () => setChecked(!checked);

  const handleKeyDown = (event: KeyboardEvent<HTMLSpanElement>) => {
    if (disabled) {
      return;
    }
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      toggle();
    }
  };

  const labelBase = isCompact
    ? 'fs-switch__label text-foreground'
    : 'text-sm font-medium leading-normal';
  const labelMuted = isCompact ? 'fs-switch__label' : 'text-muted-foreground';
  const optionClass = (active: boolean) => {
    const tone = active ? (isCompact ? 'text-foreground' : 'text-foreground') : labelMuted;
    return [
      'fs-switch__dual-option',
      'transition-colors',
      'border-0',
      'bg-transparent',
      'p-0',
      isCompact ? '' : labelBase,
      disabled ? 'opacity-50' : '',
      tone,
      disabled ? '' : 'cursor-pointer',
    ]
      .filter(Boolean)
      .join(' ');
  };

  const thumbPrimary = Boolean(optionLabels);
  const thumbStyle: CSSProperties = disabled
    ? {
        borderColor: 'var(--color-border)',
        backgroundColor: 'var(--color-neutral)',
      }
    : thumbPrimary || checked
      ? {
          borderColor: 'var(--color-primary)',
          backgroundColor: 'var(--color-primary)',
        }
      : {
          borderColor: 'var(--color-secondary)',
          backgroundColor: 'var(--color-background, #fff)',
        };

  const trackClass = [
    'fs-switch__track',
    isCompact ? 'fs-switch__track--compact' : '',
    checked ? 'fs-switch__track--checked' : '',
    disabled ? 'fs-switch__track--disabled' : '',
  ]
    .filter(Boolean)
    .join(' ');

  const track = (
    <span
      className={trackClass}
      onClick={(e) => {
        e.stopPropagation();
        toggle();
      }}
      role="switch"
      aria-checked={checked}
      aria-disabled={disabled}
      tabIndex={disabled ? -1 : 0}
      onKeyDown={handleKeyDown}
    >
      <span className="fs-switch__thumb" style={thumbStyle} />
    </span>
  );

  if (optionLabels) {
    const dualClass = [
      'fs-switch__dual',
      isCompact ? 'fs-switch__dual--compact' : '',
      disabled ? 'cursor-not-allowed' : '',
      className,
    ]
      .filter(Boolean)
      .join(' ');

    return (
      <div
        className={dualClass}
        data-test-id={props['data-test-id'] || 'switch-dual-root'}
        role="group"
        aria-label={`${optionLabels.off} o ${optionLabels.on}`}
      >
        <button
          type="button"
          disabled={disabled}
          className={optionClass(!checked)}
          onClick={() => setChecked(false)}
          aria-pressed={!checked}
        >
          {optionLabels.off}
        </button>
        {track}
        <button
          type="button"
          disabled={disabled}
          className={optionClass(checked)}
          onClick={() => setChecked(true)}
          aria-pressed={checked}
        >
          {optionLabels.on}
        </button>
      </div>
    );
  }

  const rootClass = [
    'fs-switch__root',
    isCompact ? 'fs-switch__root--compact' : '',
    disabled ? 'fs-switch__root--disabled' : '',
    className,
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <label className={rootClass} data-test-id={props['data-test-id'] || 'switch-root'}>
      {labelPosition === 'left' && label ? (
        <span className={`${labelBase} ${disabled ? 'opacity-50' : ''}`}>{label}</span>
      ) : null}
      {track}
      {labelPosition === 'right' && label ? (
        <span className={`${labelBase} ${disabled ? 'opacity-50' : ''}`}>{label}</span>
      ) : null}
    </label>
  );
};

export default Switch;
