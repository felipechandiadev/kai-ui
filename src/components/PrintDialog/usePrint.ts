'use client';

import { useRef } from 'react';
import type { PageSize, PageOrientation } from './PrintDialog.types';

const generatePageStyle = (pageSize: PageSize = 'A4', pageOrientation: PageOrientation = 'portrait'): string => {
  return `
    @page {
      size: ${pageSize} ${pageOrientation};
      margin: 8mm;
    }

    @media print {
      html,
      body {
        margin: 0;
        padding: 0;
        background: #ffffff;
        -webkit-print-color-adjust: exact;
        print-color-adjust: exact;
        color-adjust: exact;
      }
    }
  `;
};

export const usePrint = (
  fileName: string = 'document',
  pageSize: PageSize = 'A4',
  pageOrientation: PageOrientation = 'portrait',
) => {
  const contentRef = useRef<HTMLDivElement>(null);
  const pageStyle = generatePageStyle(pageSize, pageOrientation);

  const handlePrint = () => {
    const content = contentRef.current;
    if (!content || typeof window === 'undefined') return;

    // Collect existing styles (link and style tags) to include in print window
    const styles = Array.from(document.querySelectorAll('link[rel="stylesheet"], style'))
      .map((n) => n.outerHTML)
      .join('\n');

    const printWindow = window.open('', '_blank');
    if (!printWindow) return;

    printWindow.document.open();
    printWindow.document.write(`<!doctype html><html><head><title>${fileName}</title>${styles}<style>${pageStyle}</style></head><body></body></html>`);
    printWindow.document.close();

    // Clone content into print window
    const cloned = content.cloneNode(true) as HTMLElement;
    // Remove any interactive elements that shouldn't be printed (buttons)
    cloned.querySelectorAll('button').forEach((b) => b.remove());

    printWindow.document.body.appendChild(cloned);
    printWindow.focus();

    setTimeout(() => {
      try {
        printWindow.print();
        printWindow.close();
        console.log(`Printed: ${fileName}`);
      } catch (e) {
        console.error('Print failed', e);
      }
    }, 250);
  };

  return {
    contentRef,
    handlePrint,
  };
};
