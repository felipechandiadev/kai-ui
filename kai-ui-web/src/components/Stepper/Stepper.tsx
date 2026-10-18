"use client";

import React from "react";
import "./stepper.css";

export type StepperStepItem = {
  id: string;
  title: string;
  description?: string;
};

export type StepperProps = {
  steps: StepperStepItem[];
  /** Índice del paso activo (0-based). */
  activeIndex: number;
  /**
   * Contenido del paso (formularios), renderizado debajo del encabezado
   * visual del stepper.
   */
  children?: React.ReactNode;
  /**
   * Pie opcional bajo `children`. Si el Stepper va dentro de un `Dialog`, suele dejarse vacío
   * y colocar Atrás/Siguiente en `Dialog.actions` para un solo bloque de acciones en el modal
   * (ver editor de promociones en admin).
   */
  footer?: React.ReactNode;
  /** Contenido opcional alineado a la derecha en la fila superior (p. ej. título contextual). */
  headerAside?: React.ReactNode;
  /**
   * Si es true, al hacer clic en un paso ya completado (`index < activeIndex`)
   * se llama `onCompletedStepClick` con ese índice.
   */
  allowClickCompletedSteps?: boolean;
  onCompletedStepClick?: (index: number) => void;
  className?: string;
  contentClassName?: string;
  "data-test-id"?: string;
};

/**
 * Chrome presentacional para flujos multi-paso: puntos de estado,
 * título/descripción del paso actual y ranura para el cuerpo del paso.
 * No incluye lógica de formulario ni botones de navegación por defecto.
 */
export const Stepper: React.FC<StepperProps> = ({
  steps,
  activeIndex,
  children,
  footer,
  headerAside,
  allowClickCompletedSteps = false,
  onCompletedStepClick,
  className = "",
  contentClassName = "",
  "data-test-id": dataTestId,
}) => {
  const total = steps.length;
  const safeIndex = total === 0 ? 0 : Math.min(Math.max(activeIndex, 0), total - 1);
  const current = steps[safeIndex];

  return (
    <div
      className={["flex w-full flex-col gap-4", className].filter(Boolean).join(" ")}
      data-test-id={dataTestId}
    >
      {total > 0 ? (
        <nav
          className="flex flex-col gap-4"
          role="navigation"
          aria-label="Pasos del asistente"
        >
          <div className="flex items-center justify-between gap-4">
            <div className="flex flex-wrap items-center gap-2">
              {steps.map((step, index) => {
                const isActive = index === safeIndex;
                const isCompleted = index < safeIndex;
                const isClickable =
                  allowClickCompletedSteps && isCompleted && onCompletedStepClick;

                const dotClasses = [
                  "fs-stepper__dot flex h-8 w-8 shrink-0 items-center justify-center rounded-full border-2 text-xs font-semibold transition-all duration-200",
                  isActive
                    ? "border-secondary bg-secondary text-white shadow-sm"
                    : isCompleted
                      ? "border-primary/40 bg-primary/15 text-primary"
                      : "border-border bg-card text-muted-foreground",
                ].join(" ");

                return (
                  <button
                    key={step.id}
                    type="button"
                    disabled={!isClickable && !isActive}
                    aria-current={isActive ? "step" : undefined}
                    aria-label={`Paso ${index + 1}: ${step.title}`}
                    className={[
                      dotClasses,
                      isClickable ? "cursor-pointer hover:border-primary" : "",
                      !isClickable && !isActive ? "cursor-default opacity-90" : "",
                    ]
                      .filter(Boolean)
                      .join(" ")}
                    data-test-id={`stepper-dot-${step.id}`}
                    onClick={() => {
                      if (isClickable) {
                        onCompletedStepClick?.(index);
                      }
                    }}
                  >
                    {isCompleted ? (
                      <span className="text-sm leading-none" aria-hidden>
                        ✓
                      </span>
                    ) : (
                      index + 1
                    )}
                  </button>
                );
              })}
            </div>
            {headerAside ? (
              <div className="min-w-0 shrink text-right text-sm text-muted-foreground">
                {headerAside}
              </div>
            ) : null}
          </div>

          <div className="overflow-hidden rounded-xl border border-border bg-card shadow-sm">
            <div className="relative p-5 sm:p-6">
              <div className="relative min-w-0 text-right">
                {current ? (
                  <>
                    <h3 className="text-xl font-semibold tracking-tight text-foreground">
                      {current.title}
                    </h3>
                    {current.description ? (
                      <p className="mt-1 text-sm leading-relaxed text-muted-foreground">
                        {current.description}
                      </p>
                    ) : null}
                  </>
                ) : null}
              </div>
            </div>
          </div>
        </nav>
      ) : null}

      {children ? (
        <div
          className={["min-h-0 w-full flex-1", contentClassName].filter(Boolean).join(" ")}
          data-test-id={dataTestId ? `${dataTestId}-content` : undefined}
        >
          {children}
        </div>
      ) : null}

      {footer ? (
        <div
          className="border-t border-border pt-3"
          data-test-id={dataTestId ? `${dataTestId}-footer` : undefined}
        >
          {footer}
        </div>
      ) : null}
    </div>
  );
};

export default Stepper;
