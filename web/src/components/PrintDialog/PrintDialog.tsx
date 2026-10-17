'use client';

import React from 'react';
import Dialog from '../Dialog/Dialog';
import { Button } from '../Button/Button';
import IconButton from '../IconButton/IconButton';
import { usePrint } from './usePrint';
import type { PrintDialogProps } from './PrintDialog.types';
import styles from './PrintDialog.module.css';

export const PrintDialog: React.FC<PrintDialogProps> = ({
  open,
  onClose,
  children,
  title = 'Imprimir',
  fileName = 'documento',
  disablePrint = false,
  printLoading = false,
  showPrintButton = true,
  printIconButton = false,
  size = 'md',
  customSize,
  maxWidth,
  fullWidth = false,
  scroll = 'body',
  zIndex = 50,
  contentStyle,
  extraActions,
  pageSize = 'A4',
  pageOrientation = 'portrait',
}) => {
  const { contentRef, handlePrint } = usePrint(fileName, pageSize, pageOrientation);

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={title}
      size={size}
      customSize={customSize}
      maxWidth={maxWidth}
      fullWidth={fullWidth}
      scroll={scroll}
      zIndex={zIndex}
      contentStyle={contentStyle}
    >
      <div className={styles.printContainer}>
        <div ref={contentRef} className={styles.printContent}>
          {children}
        </div>

        <div className={styles.actions}>
          {extraActions && <div className={styles.extraActions}>{extraActions}</div>}
          {showPrintButton &&
            (printIconButton ? (
              <IconButton
                icon="Printer"
                variant="action"
                size="md"
                disabled={disablePrint}
                isLoading={printLoading}
                title="Imprimir"
                ariaLabel="Imprimir"
                onClick={() => {
                  handlePrint();
                }}
                data-test-id="print-dialog-print"
              />
            ) : (
              <Button
                variant="primary"
                disabled={disablePrint}
                loading={printLoading}
                onClick={() => {
                  handlePrint();
                }}
              >
                {printLoading ? 'Cargando análisis...' : '🖨️ Imprimir'}
              </Button>
            ))}
        </div>
      </div>
    </Dialog>
  );
};

export default PrintDialog;
