"use client";

import React, { useId } from "react";
import { Trash2 } from "lucide-react";
import Dialog from "./Dialog";
import { Button, ButtonPill } from "../Button";
import Alert from "../Alert/Alert";
import DotProgress from "../DotProgress/DotProgress";

export type DeleteDialogProps = {
  open: boolean;
  onClose: () => void;
  /** Llamar al confirmar (eliminar). Puede ser async. */
  onConfirm: () => void | Promise<void>;
  /** Texto o contenido de la advertencia. */
  message: React.ReactNode;
  isSubmitting?: boolean;
  title?: string;
  /** Línea secundaria bajo el título del `Dialog` (misma pauta que `DeleteBaseForm`). */
  subtitle?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  /** Mostradas en `alertArea` del `Dialog` (no en el cuerpo), alineado con reglas del admin. */
  errors?: string[];
  /** Por defecto `custom` con `maxWidth` (~24rem). Podés usar `sm` u otro preset del `Dialog`. */
  size?: "xs" | "sm" | "md" | "lg" | "xl" | "xxl" | "custom";
  maxWidth?: number | string;
  "data-test-id"?: string;
};

/**
 * Diálogo de confirmación de borrado, basado en el `Dialog` compartido.
 * Misma idea que `DeleteBaseForm` (ícono, mensaje, estados, DotProgress) pero con pie
 * `Cancelar` + `Eliminar` vía `actions` y errores en `alertArea`.
 */
export function DeleteDialog({
  open,
  onClose,
  onConfirm,
  message,
  isSubmitting = false,
  title = "Confirmar eliminación",
  subtitle,
  confirmLabel = "Eliminar",
  cancelLabel = "Cancelar",
  errors = [],
  size: dialogSize = "custom",
  maxWidth = "24rem",
  "data-test-id": dataTestId = "delete-dialog",
}: DeleteDialogProps) {
  const formId = useId();
  const formElementId = `delete-dialog-form-${formId}`;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    void onConfirm();
  };

  const alertArea =
    errors.length > 0 ? (
      <div className="flex flex-col gap-2">
        {errors.map((err, i) => (
          <Alert key={i} variant="error" data-test-id={`${dataTestId}-error-${i}`}>
            {err}
          </Alert>
        ))}
      </div>
    ) : null;

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={title}
      size={dialogSize === "custom" ? "custom" : dialogSize}
      maxWidth={dialogSize === "custom" ? maxWidth : undefined}
      scroll="body"
      actionsJustify="between"
      data-test-id={dataTestId}
      showCloseButton={false}
      alertArea={alertArea}
      actions={
        <>
          <Button
            type="button"
            variant="outlined"
            size="md"
            onClick={onClose}
            disabled={isSubmitting}
            data-test-id={`${dataTestId}-cancel`}
          >
            {cancelLabel}
          </Button>
          <ButtonPill
            type="submit"
            form={formElementId}
            variant="primary"
            disabled={isSubmitting}
            className="!bg-red-600 hover:!bg-red-700 !text-white !font-semibold"
            data-test-id={`${dataTestId}-confirm`}
          >
            {isSubmitting ? (
              <span className="inline-flex min-h-[1.25rem] items-center justify-center">
                <DotProgress size={12} />
              </span>
            ) : (
              confirmLabel
            )}
          </ButtonPill>
        </>
      }
    >
      {typeof subtitle === "string" && subtitle.trim() !== "" ? (
        <p className="mb-4 text-sm text-muted-foreground" data-test-id={`${dataTestId}-subtitle`}>
          {subtitle}
        </p>
      ) : null}

      <form id={formElementId} onSubmit={handleSubmit} className="flex flex-col">
        <div className="mb-2 flex flex-col space-y-4">
          <div className="flex justify-center">
            <div
              className="flex h-14 w-14 items-center justify-center rounded-full border border-red-200 bg-red-50"
              aria-hidden
            >
              <Trash2 size={24} strokeWidth={1.5} className="text-red-500" />
            </div>
          </div>
          <div
            className="text-center text-base leading-relaxed text-foreground"
            data-test-id={`${dataTestId}-message`}
          >
            {message}
          </div>
        </div>
      </form>
    </Dialog>
  );
}

export default DeleteDialog;
