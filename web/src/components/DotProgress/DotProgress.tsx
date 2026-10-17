'use client';

import React, { useEffect, useState } from "react";
import "./dot-progress.css";

export type DotProgressVariant = "step" | "wave";

interface DotProgressProps {
  size?: number;
  gap?: number;
  colorPrimary?: string;
  colorNeutral?: string;
  className?: string;
  interval?: number;
  /** Cantidad de bolitas. Si no se pasa, se usa `totalSteps`. */
  count?: number;
  /** Alias de `count`. El indicador de pasos del stepper lo sigue usando. */
  totalSteps?: number;
  activeStep?: number;
  /** `step` ilumina una bolita. `wave` las hace saltar en secuencia. */
  variant?: DotProgressVariant;
}

const DEFAULT_SIZE = 16;
const DEFAULT_GAP = 8;
const DEFAULT_PRIMARY = "var(--color-primary)";
const DEFAULT_NEUTRAL = "var(--color-neutral)";
const DEFAULT_INTERVAL = 350;
const WAVE_STAGGER_S = 0.16;

const DotProgress: React.FC<DotProgressProps> = ({
  size = DEFAULT_SIZE,
  gap = DEFAULT_GAP,
  colorPrimary = DEFAULT_PRIMARY,
  colorNeutral = DEFAULT_NEUTRAL,
  className = "",
  interval = DEFAULT_INTERVAL,
  count,
  totalSteps = 5,
  activeStep,
  variant = "step",
}) => {
  const steps = Math.max(1, count ?? totalSteps);
  const isWave = variant === "wave";
  const [active, setActive] = useState(activeStep ?? 0);

  useEffect(() => {
    if (isWave) return;
    if (activeStep !== undefined) {
      setActive(activeStep);
      return;
    }
    const timer = setInterval(() => {
      setActive((prev) => (prev + 1) % steps);
    }, interval);
    return () => clearInterval(timer);
  }, [interval, steps, activeStep, isWave]);

  return (
    <span
      className={`inline-flex items-center ${className}`}
      style={{ gap, paddingTop: isWave ? Math.round(size * 0.7) : undefined }}
      data-test-id="dot-progress-root"
      data-variant={variant}
      data-count={steps}
      role="status"
      aria-hidden
    >
      {Array.from({ length: steps }, (_, i) => {
        const isActive = !isWave && i === active % steps;
        const pulse = isActive && activeStep === undefined;
        return (
          <span
            key={i}
            className={
              isWave
                ? "fs-dot-progress__dot fs-dot-progress__dot--wave"
                : pulse
                  ? "fs-dot-progress__dot fs-dot-progress__dot--pulse"
                  : "fs-dot-progress__dot"
            }
            style={{
              width: size,
              height: size,
              borderRadius: "50%",
              backgroundColor: isWave || isActive ? colorPrimary : colorNeutral,
              display: "inline-block",
              flexShrink: 0,
              transition: isWave ? undefined : "background-color 0.2s",
              animationDelay: isWave ? `${i * WAVE_STAGGER_S}s` : undefined,
              cursor: "default",
            }}
          />
        );
      })}
    </span>
  );
};

export default DotProgress;
