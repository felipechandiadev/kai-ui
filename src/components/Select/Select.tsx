'use client';

import React, { useState, useEffect, useRef } from "react";
import { ChevronDown } from "lucide-react";
import DropdownList, { dropdownOptionClass } from "../DropdownList/DropdownList";
import IconButton from "../IconButton/IconButton";
import { TextField } from "../TextField/TextField";

export interface Option {
  id: string | number;
  label: string;
}

interface SelectProps {
  label?: string;
  options: Option[];
  placeholder?: string;
  value?: string | number | null;
  onChange?: (id: string | number | null) => void;
  required?: boolean;
  name?: string;
  variant?: "default" | "minimal";
  /** Misma altura que `TextField` con `density="compact"` (~2rem). */
  density?: "default" | "compact";
  /** `inline`: label y control en la misma fila (`default` o `compact`). */
  labelLayout?: "stack" | "inline";
  ["data-test-id"]?: string;
  allowClear?: boolean;
  disabled?: boolean;
  /** Oculta el icono de despliegue (ChevronDown). Útil cuando no hay opciones. */
  hideDropdownIcon?: boolean;
  className?: string;
  /** Mantiene visible la etiqueta del campo aunque no haya valor seleccionado. */
  alwaysShowLabel?: boolean;
}

const Select: React.FC<SelectProps> = ({
  label,
  options,
  placeholder,
  value = null,
  onChange,
  required = false,
  name,
  variant = "default",
  density = "default",
  labelLayout = "stack",
  allowClear = false,
  disabled = false,
  hideDropdownIcon = false,
  className = "",
  alwaysShowLabel = false,
  ...props
}) => {
  const isCompact = density === "compact";
  const [open, setOpen] = useState(false);
  const [focused, setFocused] = useState(false);
  const [isSelecting, setIsSelecting] = useState(false);
  const [highlightedIndex, setHighlightedIndex] = useState(-1);
  const selected =
    value === null || value === undefined ? undefined : options.find((opt) => String(opt.id) === String(value));
  const onChangeRef = useRef(onChange);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    onChangeRef.current = onChange;
  }, [onChange]);

  useEffect(() => {
    if (required) {
      const hiddenInput = document.querySelector(`input[name="${name || "select-validation"}"]`) as HTMLInputElement;
      if (hiddenInput) {
        if (value === null || value === undefined) {
          hiddenInput.setCustomValidity("Este campo es requerido");
        } else {
          hiddenInput.setCustomValidity("");
        }
      }
    }
  }, [value, required, name]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!focused) return;

      if (!open && ["ArrowDown", "ArrowUp", "Enter"].includes(e.key)) {
        e.preventDefault();
        setOpen(true);
        setHighlightedIndex(e.key === "ArrowUp" ? options.length - 1 : 0);
        return;
      }

      if (!open) return;

      if (e.key === "ArrowDown") {
        e.preventDefault();
        setHighlightedIndex((i) => (i < options.length - 1 ? i + 1 : 0));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setHighlightedIndex((i) => (i > 0 ? i - 1 : options.length - 1));
      } else if (e.key === "Enter" && highlightedIndex >= 0) {
        e.preventDefault();
        onChangeRef.current?.(options[highlightedIndex].id);
        setOpen(false);
        setHighlightedIndex(-1);
      } else if (e.key === "Escape") {
        e.preventDefault();
        setOpen(false);
        setHighlightedIndex(-1);
      }
    };

    if (focused) {
      document.addEventListener("keydown", handleKeyDown);
    }

    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [focused, open, options, highlightedIndex]);

  const optionRefs = useRef<(HTMLLIElement | null)[]>([]);

  useEffect(() => {
    if (open && highlightedIndex >= 0 && optionRefs.current[highlightedIndex]) {
      optionRefs.current[highlightedIndex]?.scrollIntoView({ block: "nearest" });
    }
  }, [highlightedIndex, open]);

  const hasValue = value !== null && value !== undefined;
  const hasSelection = Boolean(selected);
  /** Texto visible: siempre el label de la opción, nunca el id crudo (`value`). */
  const selectedDisplayLabel = selected ? String(selected.label) : "";
  const hasClear = allowClear && hasValue;
  const shrinkMinimal = focused || hasValue || alwaysShowLabel;
  const showMinimalCompactFloat =
    isCompact && variant === "minimal" && (Boolean(label?.trim()) || alwaysShowLabel);
  const minimalFloatCaption = (label?.trim() || placeholder?.trim() || "").trim();
  const displayTitle = selected ? String(selected.label) : (placeholder ?? "Selecciona");

  const iconBtnBase = isCompact ? "min-h-5 min-w-5 p-0" : "min-h-6 min-w-6 p-0";
  const clearBtnRight = isCompact ? "right-8" : "right-10";
  const chevronRight = hasClear ? (isCompact ? "right-3" : "right-3.5") : isCompact ? "right-2" : "right-3";

  /** Etiqueta arriba + disparador bordeado (no mete el label dentro del `h-8`). */
  const stackedCompactLabel =
    labelLayout === "stack" && variant === "default" && isCompact && Boolean(label?.trim());
  const inlineInsetLabel =
    labelLayout === "inline" && variant === "default" && Boolean(label?.trim());

  const defaultComboShellClass = (
    insetShell?: boolean,
  ) =>
    `relative w-full rounded-md border border-border focus-within:border-primary fs-text-field__combo-shell ${
      insetShell
        ? isCompact
          ? "flex h-8 min-h-8 w-full min-w-0 items-stretch box-border"
          : "flex fs-control-height-md w-full min-w-0 items-stretch"
        : isCompact
          ? "flex h-8 min-h-8 w-full min-w-0 items-stretch box-border"
          : ""
    } ${disabled ? "fs-text-field__combo-shell--muted cursor-not-allowed opacity-50" : ""}`.trim();

  function renderDefaultCombo(
    textFieldLabel: string,
    textFieldWrapClassName: string,
    insetLabel?: { text: string; required?: boolean },
  ) {
    const comboInputValue = hasSelection
      ? selectedDisplayLabel
      : alwaysShowLabel && placeholder
        ? placeholder
        : "";
    const comboPlaceholder = hasSelection ? undefined : placeholder;

    return (
      <div
        className={`${defaultComboShellClass(Boolean(insetLabel))}${
          insetLabel
            ? ` fs-inline-combo-shell flex-row${isCompact ? "" : " fs-inline-combo-shell--default"}`
            : ""
        }`}
        onFocus={() => !disabled && setFocused(true)}
        onBlur={() => {
          if (!isSelecting) {
            setTimeout(() => setOpen(false), 150);
          }
          setFocused(false);
        }}
        onClick={() => !disabled && setOpen(!open)}
        tabIndex={disabled ? -1 : 0}
        data-test-id={props["data-test-id"] || "select-root"}
        data-has-options={options.length > 0 ? "true" : "false"}
        role="combobox"
        aria-expanded={open}
        aria-required={required}
        aria-invalid={required && (value === null || value === undefined)}
        aria-controls="select-list"
      >
        <input
          type="hidden"
          value={value !== null && value !== undefined ? value.toString() : ""}
          required={required}
          onChange={() => {}}
          name={name || "select-validation"}
          tabIndex={-1}
          aria-hidden="true"
        />

        {insetLabel ? (
          <label
            className={`fs-text-field__inline-label${isCompact ? "" : " fs-text-field__inline-label--default"}`}
            {...(name?.trim() ? { htmlFor: name } : {})}
          >
            {insetLabel.text}
            {insetLabel.required ? <span className="ml-0.5 text-red-500">*</span> : null}
          </label>
        ) : null}

        <div
          className={`relative flex min-h-0 min-w-0 flex-1 items-center${insetLabel ? "" : " w-full"}`}
        >
        <TextField
          label={textFieldLabel}
          value={comboInputValue}
          onChange={() => {}}
          placeholder={comboPlaceholder}
          required={required}
          data-test-id="select-input"
          className={`${textFieldWrapClassName}${!hasSelection && placeholder ? " [&_.fs-text-field__input]:text-muted-foreground" : ""}`.trim()}
          variante="autocomplete"
          readOnly={true}
          disabled={disabled}
          alwaysShowLabel={alwaysShowLabel}
          density={density}
        />

        {allowClear && value !== null && value !== undefined && (
          <IconButton
            icon="X"
            variant="action"
            size={isCompact ? "xs" : "sm"}
            className={`absolute top-1/2 z-20 -translate-y-1/2 ${clearBtnRight} ${iconBtnBase}`}
            onClick={() => onChange?.(null)}
            aria-label="Limpiar selección"
            data-test-id="select-clear-btn"
            tabIndex={-1}
            disabled={disabled}
          />
        )}

        {!hideDropdownIcon ? (
          <IconButton
            icon="ChevronDown"
            variant="action"
            size={isCompact ? "xs" : "sm"}
            className={`absolute right-1.5 top-1/2 z-20 -translate-y-1/2 ${iconBtnBase}`}
            tabIndex={-1}
            aria-label="Desplegar opciones"
            onClick={(e: React.MouseEvent<HTMLButtonElement>) => {
              e.stopPropagation();
              !disabled && setOpen(!open);
            }}
            data-test-id="select-dropdown-icon"
            disabled={disabled}
          />
        ) : null}
        </div>

        <DropdownList
          open={open}
          testId="select-list"
          highlightedIndex={highlightedIndex}
          onHoverChange={() => {}}
          usePortal={true}
          anchorRef={containerRef}
        >
          {options.map((opt, idx) => (
            <li
              key={opt.id}
              title={String(opt.label)}
              ref={(el) => {
                optionRefs.current[idx] = el;
              }}
              className={dropdownOptionClass}
              onMouseDown={() => {
                setIsSelecting(true);
                onChange?.(opt.id);
                setOpen(false);
                setTimeout(() => setIsSelecting(false), 200);

                setTimeout(() => {
                  const hiddenInput = document.querySelector(
                    `input[name="${name || "select-validation"}"]`,
                  ) as HTMLInputElement;
                  if (hiddenInput && required) {
                    hiddenInput.setCustomValidity("");
                    const form = hiddenInput.closest("form");
                    if (form) {
                      hiddenInput.dispatchEvent(new Event("input", { bubbles: true }));
                    }
                  }
                }, 10);
              }}
              onMouseEnter={() => {
                setHighlightedIndex(idx);
              }}
              data-test-id={`select-option-${opt.id}`}
            >
              {opt.label}
            </li>
          ))}
        </DropdownList>
      </div>
    );
  }

  return (
    <div
      className={`fs-dropdown-container w-full min-w-0 ${className}`.trim()}
      ref={containerRef}
    >
      {variant === "minimal" ? (
        <>
          <div
            className={`relative w-full cursor-pointer select-none fs-text-field__combo-shell rounded-md border border-border focus-within:border-primary ${
              isCompact
                ? "flex h-8 min-h-8 max-h-8 min-w-0 items-stretch box-border"
                : "fs-control-min-height-md"
            } ${disabled ? "fs-text-field__combo-shell--muted cursor-not-allowed opacity-50" : ""}`.trim()}
            onFocus={() => !disabled && setFocused(true)}
            onBlur={() => {
              if (!isSelecting) {
                setTimeout(() => setOpen(false), 150);
              }
              setFocused(false);
            }}
            onClick={() => !disabled && setOpen(!open)}
            tabIndex={disabled ? -1 : 0}
            data-test-id={props["data-test-id"] || "select-root"}
            data-has-options={options.length > 0 ? "true" : "false"}
            role="combobox"
            aria-expanded={open}
            aria-required={required}
            aria-invalid={required && (value === null || value === undefined)}
            aria-controls="select-list"
          >
            {showMinimalCompactFloat ? (
              <label
                className={
                  `pointer-events-none absolute left-3 z-20 -top-1 px-1 text-xs font-medium leading-tight fs-text-field__floating-label text-foreground transition-all duration-300 ease-in-out ` +
                  (shrinkMinimal ? "-translate-y-1 scale-90 opacity-100" : "opacity-0")
                }
                aria-hidden={!shrinkMinimal}
              >
                {minimalFloatCaption || "\u00a0"}
                {required ? <span className="ml-1 text-red-500">*</span> : null}
              </label>
            ) : null}
            <input
              type="hidden"
              value={value !== null && value !== undefined ? value.toString() : ""}
              required={required}
              onChange={() => {}}
              name={name || "select-validation"}
              tabIndex={-1}
              aria-hidden="true"
            />

            <div
              className={`flex min-w-0 flex-1 items-center ${
                isCompact
                  ? "box-border h-full max-h-full min-h-0 px-2.5 py-0 text-[0.8125rem] leading-5"
                  : "fs-control-min-height-md px-3 py-[var(--control-padding-y-md,0.5rem)] text-sm"
              } ${hasClear ? "pr-12" : "pr-8"}`.trim()}
            >
              <span
                title={displayTitle}
                className={`flex-1 truncate leading-normal ${
                  isCompact ? "min-h-[1.125rem] text-[0.8125rem]" : "min-h-[1.25rem] text-sm"
                } ${hasValue ? "text-foreground" : "text-muted-foreground"}`}
              >
                {selected ? selected.label : placeholder ?? "Selecciona"}
              </span>
            </div>

            {allowClear && value !== null && value !== undefined && (
              <IconButton
                icon="X"
                variant="action"
                size={isCompact ? "xs" : "sm"}
                className={`absolute top-1/2 z-20 -translate-y-1/2 ${clearBtnRight} ${iconBtnBase}`}
                onClick={() => onChange?.(null)}
                aria-label="Limpiar selección"
                data-test-id="select-clear-btn"
                tabIndex={-1}
                disabled={disabled}
              />
            )}

            <span
              className={`pointer-events-none absolute ${chevronRight} top-1/2 flex h-4 w-4 -translate-y-1/2 items-center justify-center text-base text-secondary transition-colors`}
              aria-hidden="true"
            >
              <ChevronDown className="h-4 w-4" strokeWidth={2} />
            </span>

            <DropdownList
              open={open}
              testId="select-list"
              highlightedIndex={highlightedIndex}
              onHoverChange={() => {}}
              usePortal={true}
              anchorRef={containerRef}
            >
              {options.map((opt, idx) => (
                <li
                  key={opt.id}
                  title={String(opt.label)}
                  ref={(el) => {
                    optionRefs.current[idx] = el;
                  }}
                  className={dropdownOptionClass}
                  onMouseDown={() => {
                    setIsSelecting(true);
                    onChange?.(opt.id);
                    setOpen(false);
                    setTimeout(() => setIsSelecting(false), 200);

                    setTimeout(() => {
                      const hiddenInput = document.querySelector(
                        `input[name="${name || "select-validation"}"]`,
                      ) as HTMLInputElement;
                      if (hiddenInput && required) {
                        hiddenInput.setCustomValidity("");
                        const form = hiddenInput.closest("form");
                        if (form) {
                          hiddenInput.dispatchEvent(new Event("input", { bubbles: true }));
                        }
                      }
                    }, 10);
                  }}
                  onMouseEnter={() => {
                    setHighlightedIndex(idx);
                  }}
                  data-test-id={`select-option-${opt.id}`}
                >
                  {opt.label}
                </li>
              ))}
            </DropdownList>
          </div>
        </>
      ) : (
        <>
          {inlineInsetLabel ? (
            renderDefaultCombo("", "min-h-0 min-w-0 flex-1 pr-16", {
              text: label ?? "",
              required,
            })
          ) : stackedCompactLabel ? (
            <div className="flex min-w-0 flex-col gap-1">
              <label
                className="text-[11px] font-medium leading-tight text-foreground"
                {...(name?.trim() ? { htmlFor: name } : {})}
              >
                {label}
                {required ? <span className="ml-1 text-red-500">*</span> : null}
              </label>
              {renderDefaultCombo("", "min-h-0 min-w-0 flex-1 pr-16")}
            </div>
          ) : (
            renderDefaultCombo(
              label ?? "",
              `min-h-0 min-w-0 ${isCompact ? "flex-1" : ""} pr-16`.trim(),
            )
          )}
        </>
      )}
    </div>
  );
};
export default Select;
