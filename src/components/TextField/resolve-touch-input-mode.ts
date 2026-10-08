export type TouchInputMode =
  | "none"
  | "text"
  | "tel"
  | "url"
  | "email"
  | "numeric"
  | "decimal"
  | "search";

export type ResolveTouchInputModeParams = {
  type: string;
  inputMode?: TouchInputMode;
  isCoarsePointer: boolean;
  allowDecimalComma?: boolean;
  allowLetters?: boolean;
  step?: number | string;
};

function stepAllowsDecimal(step: number | string | undefined): boolean {
  if (step == null || step === "") return false;
  const n = typeof step === "number" ? step : Number(step);
  if (Number.isFinite(n)) return !Number.isInteger(n);
  return String(step).includes(".") || String(step).includes(",");
}

export function resolveTouchInputMode({
  type,
  inputMode,
  isCoarsePointer,
  allowDecimalComma = false,
  allowLetters = false,
  step,
}: ResolveTouchInputModeParams): TouchInputMode | undefined {
  if (inputMode) return inputMode;
  if (!isCoarsePointer) return undefined;

  switch (type) {
    case "currency":
      return allowDecimalComma ? "decimal" : "numeric";
    case "number":
      return stepAllowsDecimal(step) ? "decimal" : "numeric";
    case "datePicker":
      return "numeric";
    case "tel":
      return allowLetters ? undefined : "tel";
    default:
      return undefined;
  }
}

/** En touch, `type="number"` nativo muestra spinners raros; preferir text + inputMode. */
export function shouldUseTextInputForNumericType(
  type: string,
  isCoarsePointer: boolean,
  selectOnFocusForNumber: boolean,
): boolean {
  if (type === "number" && selectOnFocusForNumber) return true;
  if (!isCoarsePointer) return false;
  return type === "number" || type === "datePicker";
}
