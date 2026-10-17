"use client";
import React, { useCallback, useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import Dialog from './Dialog';
import { Button } from '../Button/Button';

interface DialogToPrintProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: React.ReactNode;
  /**
   * Optional size forwarded to the base Dialog component.
   * Defaults to `lg` which gives a comfortable width for printable content.
   */
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl' | 'custom';
  /**
   * Text for the print action button.
   */
  printLabel?: string;
  /**
   * Text for the close action button.
   */
  closeLabel?: string;
  /**
   * Optional className applied to the internal printable container.
   */
  contentClassName?: string;
  /**
   * Called right before opening the print preview.
   */
  onBeforePrint?: () => void;
  /**
   * Called after the print window is closed.
   */
  onAfterPrint?: () => void;
  /**
   * Z-index for the dialog. Defaults to 50 (same as base Dialog) so it can be overridden when needed.
   */
  zIndex?: number;
  /**
   * Optional DOM element to render the dialog into. Defaults to a body-level portal for better layering.
   */
  portalContainer?: HTMLElement | null;
  /**
   * Force using the browser print dialog instead of Electron silent printing when available.
   */
  preferBrowserPrint?: boolean;
  /**
   * Scroll behavior: 'body' keeps page scrollable; 'paper' enables internal scroller.
   * Defaults to 'paper' for better print layout control.
   */
  scroll?: 'body' | 'paper';
  /**
   * Extra CSS appended inside the print document head.
   */
  printStyles?: string;
}

const DialogToPrint: React.FC<DialogToPrintProps> = ({
  open,
  onClose,
  title,
  children,
  size = 'lg',
  printLabel = 'Imprimir',
  closeLabel = 'Cerrar',
  contentClassName = '',
  onBeforePrint,
  onAfterPrint,
  zIndex = 50,
  portalContainer,
  preferBrowserPrint = false,
  scroll = 'paper',
  printStyles,
}) => {
  const printableRef = useRef<HTMLDivElement | null>(null);
  const [defaultPortalElement, setDefaultPortalElement] = useState<HTMLElement | null>(null);
  const isBrowser = typeof window !== 'undefined';

  useEffect(() => {
    if (!isBrowser || portalContainer) {
      setDefaultPortalElement(null);
      return;
    }

    const element = document.createElement('div');
    element.className = 'dialog-to-print-portal';
    document.body.appendChild(element);
    setDefaultPortalElement(element);

    return () => {
      document.body.removeChild(element);
    };
  }, [isBrowser, portalContainer]);

  const portalTarget = portalContainer ?? defaultPortalElement;

  const buildPrintableHtml = useCallback(() => {
    const content = printableRef.current;
    if (!content) {
      return null;
    }

    const headNodes = document.querySelectorAll('link[rel="stylesheet"], style');
    const styles = Array.from(headNodes)
      .map((node) => node.outerHTML)
      .join('\n');

    const baseHref = typeof window !== 'undefined' ? `${window.location.origin}/` : '/';

    const inlinePrintStyles = `
@media print {
  body {
    margin: 0;
    padding: 0;
  }
}
${printStyles ?? ''}`;

    return `<!DOCTYPE html>
<html lang="es">
<head>
<meta charSet="utf-8" />
<title>${title ?? 'Documento'}</title>
<base href="${baseHref}" />
${styles}
<style>
${inlinePrintStyles}
</style>
</head>
<body>
<div id="print-root">${content.innerHTML}</div>
</body>
</html>`;
  }, [title]);

  const printWithIframe = useCallback((html: string, skipBeforeHook = false) => {
    if (!skipBeforeHook) {
      onBeforePrint?.();
    }

    const iframe = document.createElement('iframe');
    iframe.style.position = 'fixed';
    iframe.style.right = '0';
    iframe.style.bottom = '0';
    iframe.style.width = '0';
    iframe.style.height = '0';
    iframe.style.border = '0';
    iframe.setAttribute('aria-hidden', 'true');
    document.body.appendChild(iframe);

    const cleanup = () => {
      iframe.parentNode?.removeChild(iframe);
      onAfterPrint?.();
    };

    const iframeWindow = iframe.contentWindow;
    if (!iframeWindow) {
      cleanup();
      return;
    }
    iframe.onload = () => {
      iframeWindow.onafterprint = () => {
        cleanup();
        iframeWindow.onafterprint = null;
      };

      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          try {
            iframeWindow.focus();
            iframeWindow.print();
          } catch (error) {
            cleanup();
          }
        });
      });
    };

    iframe.srcdoc = html;
  }, [onAfterPrint, onBeforePrint]);

  const handlePrint = useCallback(async () => {
    const printableHtml = buildPrintableHtml();
    if (!printableHtml) {
      return;
    }

    const electronAPI = typeof window !== 'undefined' ? (window as any).electronAPI : undefined;

    if (preferBrowserPrint) {
      printWithIframe(printableHtml);
      return;
    }

    if (electronAPI?.printHtml) {
      onBeforePrint?.();
      try {
        const result = await electronAPI.printHtml({
          html: printableHtml,
          title: title ?? 'Documento',
          printBackground: true,
        });

        if (!result?.success) {
          console.warn('[DialogToPrint] Silent print falló, usando método alternativo:', result?.error);
          printWithIframe(printableHtml, true);
        } else {
          onAfterPrint?.();
        }
        return;
      } catch (error) {
        console.error('[DialogToPrint] Error en impresión silenciosa, usando fallback:', error);
        printWithIframe(printableHtml, true);
        return;
      }
    }

    printWithIframe(printableHtml);
  }, [buildPrintableHtml, onAfterPrint, onBeforePrint, printWithIframe, title]);

  const dialogContent = (
    <Dialog
      open={open}
      onClose={onClose}
      title={title}
      size={size}
      hideActions
      scroll={scroll}
      zIndex={zIndex}
    >
      <div
        ref={printableRef}
        className={`print-dialog-content ${contentClassName}`.trim()}
        data-test-id="print-dialog-content"
      >
        {children}
      </div>

      <div className="mt-6 flex justify-end gap-3" data-test-id="print-dialog-actions">
        <Button variant="outlined" onClick={onClose}>
          {closeLabel}
        </Button>
        <Button variant="primary" onClick={handlePrint}>
          {printLabel}
        </Button>
      </div>
    </Dialog>
  );
  
  if (!isBrowser) {
    return null;
  }

  if (!portalTarget) {
    return null;
  }

  return createPortal(dialogContent, portalTarget);
};

export default DialogToPrint;
