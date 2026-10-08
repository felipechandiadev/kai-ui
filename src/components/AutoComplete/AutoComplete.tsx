'use client'
import React, { useState, useRef, useEffect, useMemo, useCallback } from "react";
import DropdownList, { dropdownOptionClass } from "../DropdownList/DropdownList";
import IconButton from "../IconButton/IconButton";
import { TextField } from "../TextField/TextField";


export interface Option {
  id: string | number;
  label: string;
}

interface AutoCompleteProps<T = Option> {
  options: T[];
  label?: string;
  placeholder?: string;
  value?: T | null;
  onChange?: (option: T | null) => void;
  onInputChange?: (value: string) => void;
  name?: string;
  required?: boolean;
  getOptionLabel?: (option: T) => string;
  getOptionValue?: (option: T) => any;
  filterOption?: (option: T, inputValue: string) => boolean;
  ["data-test-id"]?: string;
  disabled?: boolean;
  /** Etiqueta flotante siempre visible (útil con placeholder vacío o como selector). */
  alwaysShowLabel?: boolean;
  /** Misma altura que `TextField` con `density="compact"` (~2rem). */
  density?: "default" | "compact";
}

const AutoComplete = <T = Option,>({
  options,
  label,
  placeholder,
  value = null,
  onChange,
  onInputChange,
  name,
  required,
  getOptionLabel,
  getOptionValue,
  filterOption,
  alwaysShowLabel = false,
  density = "default",
  ...props
}: AutoCompleteProps<T>) => {
  // Helper functions with defaults for backward compatibility
  const defaultGetOptionLabel = (option: T): string => {
    if (typeof option === 'string') return option;
    if (option && typeof option === 'object' && 'label' in option) {
      return (option as any).label;
    }
    return String(option);
  };

  const defaultGetOptionValue = (option: T): any => {
    if (typeof option === 'string') return option;
    if (option && typeof option === 'object' && 'id' in option) {
      return (option as any).id;
    }
    return option;
  };

  const getLabel = getOptionLabel || defaultGetOptionLabel;
  const getValue = getOptionValue || defaultGetOptionValue;
  const [highlightedIndex, setHighlightedIndex] = useState<number>(-1);
  const [inputValue, setInputValue] = useState(value ? getLabel(value) : "");
  const [focused, setFocused] = useState(false);
  const [open, setOpen] = useState(false);
  const [validationTriggered, setValidationTriggered] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const blurCloseTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isSelectingOptionRef = useRef(false);
  /** Refs por instancia (evita colisiones entre múltiples AutoComplete en la página). */
  const itemRefsRef = useRef<Map<string | number, HTMLLIElement | null>>(new Map());
  const disabled = (props as any).disabled;
  const isCompact = density === "compact";

  // Sync inputValue with value prop
  useEffect(() => {
    setInputValue(value ? getLabel(value) : "");
  }, [value]);

  const shrink = focused || inputValue.length > 0;
  const filteredOptions = useMemo(
    () =>
      options.filter((opt) => {
        if (typeof filterOption === "function") {
          return filterOption(opt, inputValue);
        }
        return getLabel(opt).toLowerCase().includes(inputValue.toLowerCase());
      }),
    [options, inputValue, filterOption, getLabel],
  );

  // Mantener el índice destacado válido al cambiar el filtrado/abierto.
  useEffect(() => {
    if (!open) {
      return;
    }
    if (filteredOptions.length === 0) {
      if (highlightedIndex !== -1) {
        setHighlightedIndex(-1);
      }
      return;
    }
    if (highlightedIndex >= filteredOptions.length) {
      setHighlightedIndex(filteredOptions.length - 1);
    }
  }, [open, filteredOptions.length, highlightedIndex]);

  const handleSelect = useCallback(
    (option: T) => {
      setInputValue(getLabel(option));
      setOpen(false);
      setHighlightedIndex(-1);
      onChange?.(option);
    },
    [getLabel, onChange],
  );

  const handleInputKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      if (disabled) {
        return;
      }

      const list = filteredOptions;
      const hasItems = list.length > 0;

      // Abrir con teclado aunque aún no esté abierto.
      if (!open) {
        if (e.key === "ArrowDown" || e.key === "ArrowUp" || e.key === "Enter") {
          if (!hasItems) {
            return;
          }
          e.preventDefault();
          setOpen(true);
          setHighlightedIndex(e.key === "ArrowUp" ? list.length - 1 : 0);
        }
        return;
      }

      if (e.key === "Escape") {
        e.preventDefault();
        setOpen(false);
        setHighlightedIndex(-1);
        return;
      }

      if (!hasItems) {
        return;
      }

      if (e.key === "ArrowDown") {
        e.preventDefault();
        e.stopPropagation();
        setHighlightedIndex((i) => (i < list.length - 1 ? i + 1 : 0));
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        e.stopPropagation();
        setHighlightedIndex((i) => (i > 0 ? i - 1 : list.length - 1));
        return;
      }
      if (e.key === "Enter") {
        // Evitar submit de formularios al seleccionar.
        if (highlightedIndex >= 0 && highlightedIndex < list.length) {
          e.preventDefault();
          e.stopPropagation();
          handleSelect(list[highlightedIndex]!);
        }
      }
    },
    [disabled, filteredOptions, open, highlightedIndex, handleSelect],
  );

  /**
   * Respaldo: manejar flechas/Enter aun si el foco no está exactamente en el <input>
   * (p. ej. foco en el wrapper o en un botón dentro del control).
   */
  useEffect(() => {
    const onDocKeyDown = (e: KeyboardEvent) => {
      if (disabled) return;
      if (e.defaultPrevented) return;
      const root = containerRef.current;
      if (!root) return;
      const active = document.activeElement;
      if (!active || !root.contains(active)) return;

      // Si el foco está en el input/textarea, el handler `onKeyDown` del TextField
      // ya se encarga de la navegación. Evita doble incremento (salta opciones).
      const tag = (active as HTMLElement).tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") {
        return;
      }

      const list = filteredOptions;
      const hasItems = list.length > 0;

      if (!open) {
        if (e.key === "ArrowDown" || e.key === "ArrowUp" || e.key === "Enter") {
          if (!hasItems) return;
          e.preventDefault();
          setOpen(true);
          setHighlightedIndex(e.key === "ArrowUp" ? list.length - 1 : 0);
        }
        return;
      }

      if (e.key === "Escape") {
        e.preventDefault();
        setOpen(false);
        setHighlightedIndex(-1);
        return;
      }

      if (!hasItems) return;

      if (e.key === "ArrowDown") {
        e.preventDefault();
        setHighlightedIndex((i) => (i < list.length - 1 ? i + 1 : 0));
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setHighlightedIndex((i) => (i > 0 ? i - 1 : list.length - 1));
        return;
      }
      if (e.key === "Enter") {
        if (highlightedIndex >= 0 && highlightedIndex < list.length) {
          e.preventDefault();
          handleSelect(list[highlightedIndex]!);
        }
      }
    };

    // Bubble: así el input puede `stopPropagation()` y evitar dobles eventos.
    document.addEventListener("keydown", onDocKeyDown);
    return () => document.removeEventListener("keydown", onDocKeyDown);
  }, [disabled, filteredOptions, open, highlightedIndex, handleSelect]);

  // Scroll al item destacado (auto evita saltos bruscos con listas largas)
  useEffect(() => {
    if (highlightedIndex >= 0 && open) {
      const highlightedKey = getValue(filteredOptions[highlightedIndex]);
      const element = itemRefsRef.current.get(highlightedKey);
      if (element) {
        element.scrollIntoView({ block: "nearest" });
      }
    }
  }, [highlightedIndex, open, filteredOptions, getValue]);

  const handleValidation = () => {
    if (required && (!value || (inputValue && !value))) {
      setValidationTriggered(true);
      setOpen(false); // Prevent dropdown from opening when validation fails
    } else {
      setValidationTriggered(false);
    }
  };

  const focusInput = useCallback(() => {
    containerRef.current
      ?.querySelector<HTMLInputElement>('input[data-test-id="auto-complete-input"]')
      ?.focus();
  }, []);

  const cancelScheduledClose = useCallback(() => {
    if (blurCloseTimerRef.current != null) {
      clearTimeout(blurCloseTimerRef.current);
      blurCloseTimerRef.current = null;
    }
  }, []);

  const scheduleClose = useCallback(() => {
    cancelScheduledClose();
    blurCloseTimerRef.current = setTimeout(() => {
      blurCloseTimerRef.current = null;
      if (isSelectingOptionRef.current) {
        return;
      }
      const active = document.activeElement;
      if (active && containerRef.current?.contains(active)) {
        return;
      }
      setOpen(false);
    }, 150);
  }, [cancelScheduledClose]);

  useEffect(() => () => cancelScheduledClose(), [cancelScheduledClose]);

  /** Evita blur del input al pulsar chevron / limpiar (causa parpadeo abrir-cerrar). */
  const handleChromeMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    cancelScheduledClose();
  };

  const handleChevronClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    cancelScheduledClose();
    focusInput();
    if (filteredOptions.length === 0) {
      setOpen(false);
      return;
    }
    setOpen((prev) => !prev);
  };

  const handleContainerBlur = (e: React.FocusEvent) => {
    const next = e.relatedTarget as Node | null;
    if (next && containerRef.current?.contains(next)) {
      return;
    }
    setFocused(false);
    handleValidation();
    setHighlightedIndex(-1);
    if (!isSelectingOptionRef.current) {
      scheduleClose();
    }
  };

  const handleContainerFocusCapture = (e: React.FocusEvent) => {
    if (disabled) {
      return;
    }
    const target = e.target as HTMLElement;
    if (
      target.closest('[data-test-id="auto-complete-dropdown-icon"]') ||
      target.closest('[data-test-id="auto-complete-clear-icon"]')
    ) {
      return;
    }
    setFocused(true);
    if (filteredOptions.length > 0) {
      setOpen(true);
    }
  };

  const handleClear = () => {
    setInputValue(""); // Clear the input text
    setOpen(false); // Close the dropdown
    setHighlightedIndex(-1); // Reset the highlighted index
    onInputChange?.("");
    onChange?.(null); // Clear the selected option
  };

  const compactFloatCaption = (label?.trim() || placeholder?.trim() || "").trim();
  const useCompactFloatingLabel =
    isCompact &&
    (Boolean(label?.trim()) || alwaysShowLabel || Boolean(placeholder?.trim()));

  return (
    <div className="fs-dropdown-container" ref={containerRef} data-test-id={props["data-test-id"] || "auto-complete-root"} data-has-options={options.length > 0 ? "true" : "false"}>
      <div
        className={`relative w-full min-w-0 fs-text-field__combo-shell rounded-md border border-border focus-within:border-primary ${
          isCompact
            ? "flex h-8 min-h-8 max-h-8 items-stretch box-border"
            : "flex fs-control-height-md items-stretch"
        } ${disabled ? "fs-text-field__combo-shell--muted cursor-not-allowed opacity-50" : ""}`.trim()}
        onFocusCapture={handleContainerFocusCapture}
        onBlur={handleContainerBlur}
        tabIndex={-1}
      >
        <TextField
          label={useCompactFloatingLabel ? compactFloatCaption : (label || "")}
          value={inputValue}
          onChange={e => {
            const newValue = e.target.value;
            setInputValue(newValue);
            onInputChange?.(newValue);
            setOpen(true);
            setHighlightedIndex(-1);
          }}
          onKeyDown={handleInputKeyDown}
          placeholder={placeholder}
          name={name}
          required={required}
          data-test-id="auto-complete-input"
          className={`${isCompact ? "min-h-0 min-w-0 flex-1" : "w-full"} pr-20`.trim()}
          variante="autocomplete"
          disabled={disabled}
          alwaysShowLabel={
            useCompactFloatingLabel
              ? alwaysShowLabel || !label?.trim()
              : alwaysShowLabel
          }
          density={density}
        />

        {value && !disabled && (
          <IconButton
            icon="X"
            variant="action"
            className="absolute right-10 top-1/2 z-20 flex h-6 w-6 min-h-6 min-w-6 -translate-y-1/2 items-center justify-center p-0"
            onMouseDown={handleChromeMouseDown}
            onClick={handleClear}
            aria-label="Limpiar selección"
            data-test-id="auto-complete-clear-icon"
            tabIndex={-1}
            disabled={disabled}
          />
        )}

        {!disabled && (
          <IconButton
            icon="ChevronDown"
            variant="action"
            className="absolute right-2 top-1/2 z-20 flex h-6 w-6 min-h-6 min-w-6 -translate-y-1/2 items-center justify-center p-0"
            tabIndex={-1}
            aria-label="Desplegar opciones"
            onMouseDown={handleChromeMouseDown}
            onClick={handleChevronClick}
            data-test-id="auto-complete-dropdown-icon"
            disabled={disabled}
          />
        )}
      </div>
      <DropdownList 
        open={open && filteredOptions.length > 0} 
        testId="auto-complete-list"
        highlightedIndex={highlightedIndex}
        onHoverChange={(idx) => {
          // DropdownList now handles hover, we just track it if needed
        }}
        usePortal={true}
        anchorRef={containerRef}
      >
        {filteredOptions.map((opt, idx) => {
          const optValue = getValue(opt);
          const isHighlighted = highlightedIndex === idx;
          return (
            <li
              key={optValue}
              ref={(el) => {
                if (el) itemRefsRef.current.set(optValue, el);
                else itemRefsRef.current.delete(optValue);
              }}
              className={dropdownOptionClass}
              onMouseDown={() => {
                isSelectingOptionRef.current = true;
                cancelScheduledClose();
                handleSelect(opt);
                window.setTimeout(() => {
                  isSelectingOptionRef.current = false;
                }, 200);
              }}
              onMouseEnter={() => {
                setHighlightedIndex(idx);
              }}
              role="option"
              aria-selected={isHighlighted}
              data-test-id={`auto-complete-option-${optValue}`}
            >
              {getLabel(opt)}
            </li>
          );
        })}
      </DropdownList>
    </div>
  );
};

export default AutoComplete;
