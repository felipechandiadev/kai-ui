"use client";
import React, { useState, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { Button } from '../Button/Button';
import './dialog.css';

interface DialogProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: React.ReactNode;
  // Preset sizes. 'custom' allows using maxWidth prop.
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl' | 'xxl' | 'custom';
  // Custom width configuration per breakpoint (overrides size presets)
  customSize?: Partial<Record<BreakpointKey, number>>;
  // If provided and size === 'custom', overrides dialog max width. number -> px or string with units
  maxWidth?: number | string;
  // Enable full width on all screens
  fullWidth?: boolean;
  // Minimum width for the dialog
  minWidth?: number | string;
  // scroll behavior: 'body' keeps page scrollable; 'paper' enables internal scroller
  scroll?: 'body' | 'paper';
  // Height control props
  height?: number | string;
  maxHeight?: number | string;
  minHeight?: number | string;
  // Animation duration in milliseconds
  animationDuration?: number;
  // Overflow behavior
  overflowBehavior?: 'visible' | 'hidden' | 'auto';
  // Z-index override
  zIndex?: number;
  // Disable closing on backdrop click
  disableBackdropClick?: boolean;
  // Make dialog persistent (no ESC or backdrop close)
  persistent?: boolean;
  // Extra classes for outer content wrapper
  className?: string;
  // Inline style forwarded to dialog content (useful for child-specific widths)
  contentStyle?: React.CSSProperties;
  /**
   * Zona fija entre el cuerpo (`children`) y el pie (`actions`):
   * errores, avisos y cualquier `Alert` (success, info, warning, error).
   * No forma parte del scroll del cuerpo cuando `scroll="paper"`.
   */
  alertArea?: React.ReactNode;
  // Actions to display in the footer (rendered in a row with justify-between by default)
  actions?: React.ReactNode;
  // Hide the actions area (useful when actions are handled internally by children)
  hideActions?: boolean;
  // Layout for the actions row: default space-between (edges). Use "end" to align all buttons to the right.
  actionsJustify?: 'between' | 'start' | 'end' | 'center';
  // Show close button in the top-right corner
  showCloseButton?: boolean;
  /** Sin fondo propio en la franja del título (misma superficie visual que el papel del diálogo). */
  headerTransparent?: boolean;
  // Text for the close button (default: "cerrar")
  closeButtonText?: string;
  // Callback when close button is clicked (in addition to onClose)
  onCloseButtonClick?: () => void;
  // Test ID for testing purposes
  'data-test-id'?: string;
}

type DialogSizeKey = 'xs' | 'sm' | 'md' | 'lg' | 'xl' | 'xxl';
type BreakpointKey = 'xs' | 'sm' | 'md' | 'lg' | 'xl';

// Preset widths (in pixels) for each size at each breakpoint
const dialogSizePresets: Record<DialogSizeKey, Record<BreakpointKey, number>> = {
  xs: {
    xs: 280,
    sm: 320,
    md: 420,
    lg: 420,
    xl: 420,
  },
  sm: {
    xs: 320,
    sm: 420,
    md: 520,
    lg: 640,
    xl: 640,
  },
  md: {
    xs: 360,
    sm: 520,
    md: 640,
    lg: 800,
    xl: 800,
  },
  lg: {
    xs: 400,
    sm: 640,
    md: 800,
    lg: 900,
    xl: 900,
  },
  xl: {
    xs: 450,
    sm: 800,
    md: 900,
    lg: 1000,
    xl: 1024,
  },
  xxl: {
    xs: 585,
    sm: 1040,
    md: 1170,
    lg: 1300,
    xl: 1331,
  },
};

const Dialog: React.FC<DialogProps> = ({
  open,
  onClose,
  title,
  children,
  size = 'md',
  customSize,
  maxWidth,
  fullWidth = false,
  minWidth,
  scroll = 'body',
  height,
  maxHeight,
  minHeight,
  animationDuration = 200,
  overflowBehavior = 'auto',
  zIndex = 50,
  disableBackdropClick = false,
  persistent = false,
  className = '',
  contentStyle,
  alertArea,
  actions,
  hideActions = false,
  actionsJustify = 'between',
  showCloseButton = false,
  headerTransparent = false,
  closeButtonText = 'cerrar',
  onCloseButtonClick,
  'data-test-id': dataTestId,
}) => {
  const [isVisible, setIsVisible] = useState(false);
  const [shouldRender, setShouldRender] = useState(false);
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  useEffect(() => {
    if (open) {
      setShouldRender(true);
      setTimeout(() => setIsVisible(true), 10);
    } else {
      setIsVisible(false);
      setTimeout(() => setShouldRender(false), animationDuration);
    }
  }, [open, animationDuration]);

  // Bloquear/restaurar scroll del body cuando el dialog se abre/cierra
  useEffect(() => {
    if (open) {
      // Guardar el overflow original del body
      const originalOverflow = document.body.style.overflow;
      document.body.style.overflow = 'hidden';
      // Restaurar al desmontar o cerrar
      return () => {
        document.body.style.overflow = originalOverflow;
      };
    }
  }, [open]);

  // Handle ESC key
  useEffect(() => {
    if (!open || persistent) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose, persistent]);

  // Build width style based on size or customSize
  const buildWidthStyle = (): React.CSSProperties => {
    const breakpoints: BreakpointKey[] = ['xs', 'sm', 'md', 'lg', 'xl'];
    
    // For custom size, use 'md' as base or customSize if provided
    const baseSize = size === 'custom' ? 'md' : size;
    const baseWidths = dialogSizePresets[baseSize as DialogSizeKey];
    
    const widths = customSize 
      ? { ...baseWidths, ...customSize }
      : baseWidths;

    // Get width for current breakpoint (this is a simplification - real implementation would use useMediaQuery)
    // For now, we'll use the lg width as default and let CSS handle responsive
    const defaultWidth = widths['lg'] || widths['md'] || widths['sm'] || widths['xs'] || 600;
    
    return {
      width: fullWidth ? '100%' : `${defaultWidth}px`,
    };
  };

  const rootScrollClasses = `flex justify-center min-h-screen ${
    scroll === 'body' ? 'items-start overflow-y-auto pt-24 pb-12' : 'items-center'
  }`;

  const widthStyle = buildWidthStyle();
  const marginClasses = fullWidth ? 'mx-4 sm:mx-4 md:mx-4' : 'mx-4 sm:mx-8 md:mx-12';

  const contentClass = [
    'fs-dialog__paper',
    'p-0 overflow-hidden',
    marginClasses,
    'relative',
    scroll === 'body' ? 'max-h-none' : 'flex flex-col max-h-[90vh]',
    'transition-all',
    isVisible ? 'scale-100 opacity-100' : 'scale-95 opacity-0',
    className,
  ]
    .filter(Boolean)
    .join(' ');

  const contentWrapperStyle: React.CSSProperties = {
    ...widthStyle,
    maxWidth: size === 'custom' && maxWidth
      ? typeof maxWidth === 'number'
        ? `${maxWidth}px`
        : maxWidth
      : undefined,
    minWidth: minWidth ? (typeof minWidth === 'number' ? `${minWidth}px` : minWidth) : undefined,
    height: height ? (typeof height === 'number' ? `${height}px` : height) : undefined,
    maxHeight: maxHeight ? (typeof maxHeight === 'number' ? `${maxHeight}px` : maxHeight) : undefined,
    minHeight: minHeight ? (typeof minHeight === 'number' ? `${minHeight}px` : minHeight) : undefined,
    overflow: overflowBehavior,
    transitionDuration: `${animationDuration}ms`,
    ...contentStyle,
  };

  const backdropStyle: React.CSSProperties = {
    zIndex,
    transitionDuration: `${animationDuration}ms`,
  };

  const handleCloseButtonClick = () => {
    if (onCloseButtonClick) {
      onCloseButtonClick();
    }
    onClose();
  };

  const handleBackdropClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget && !disableBackdropClick && !persistent) {
      onClose();
    }
  };

  if (!shouldRender || !mounted) return null;

  const hasAlertArea = Boolean(alertArea);

  return createPortal(
    <div
      role="dialog"
      aria-modal="true"
      aria-label={title || 'Dialog'}
      className={`fixed inset-0 transition-all bg-black/70 ${isVisible ? 'opacity-100' : 'opacity-0'} ${rootScrollClasses}`}
      style={backdropStyle}
      onClick={handleBackdropClick}
      data-test-id="dialog-root"
    >
      <div
        className={contentClass}
        style={contentWrapperStyle}
        onClick={(e) => e.stopPropagation()}
        data-test-id={dataTestId || 'dialog-content'}
      >
        {title && title !== '' && (
          <div
            className={`fs-dialog__header flex items-center gap-2 border-b border-border/70 px-4 pt-4 pb-3${
              headerTransparent ? ' fs-dialog__header--transparent' : ''
            }`}
          >
            <h2
              className="title m-0 flex-1 text-left text-lg font-semibold text-foreground"
              data-test-id="dialog-title"
            >
              {title}
            </h2>
            {showCloseButton && (
              <div className="flex justify-end w-auto">
                <Button
                  variant="outlined"
                  size="sm"
                  onClick={handleCloseButtonClick}
                  className="ml-2"
                >
                  {closeButtonText}
                </Button>
              </div>
            )}
          </div>
        )}

        <div
          className={`w-full ${
            title && title !== ''
              ? 'px-4 pb-4 pt-3'
              : 'p-4'
          } ${scroll === 'paper' ? 'fs-dialog__body--paper min-h-0 flex-1 overflow-y-auto overflow-x-hidden' : ''}`}
          data-test-id="dialog-body"
        >
          {children}
        </div>

        {/* Bloque propio (hermano del cuerpo y de la fila de botones), no forma parte de `actions`. */}
        {hasAlertArea && (
          <div
            className="fs-dialog__alert-area px-4 py-3"
            data-dialog-section="alert-area"
            data-test-id="dialog-alert-area"
            role="region"
            aria-label="Avisos"
          >
            {alertArea}
          </div>
        )}

        {!hideActions && actions && (
          <div
            className={`fs-dialog__actions flex w-full flex-wrap items-center gap-2 border-t border-border/70 px-4 pb-4 pt-3 ${
              hasAlertArea ? 'mt-2' : ''
            } ${
              actionsJustify === 'between'
                ? 'justify-between'
                : actionsJustify === 'start'
                  ? 'justify-start'
                  : actionsJustify === 'end'
                    ? 'justify-end'
                    : 'justify-center'
            }`}
            data-dialog-section="actions"
            data-test-id="dialog-actions"
            role="group"
            aria-label="Acciones"
          >
            {actions}
          </div>
        )}
      </div>
    </div>,
    document.body
  );
};

export default Dialog;
