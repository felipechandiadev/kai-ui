import { describe, expect, it } from "vitest";
import {
  resolveTouchInputMode,
  shouldUseTextInputForNumericType,
} from "./resolve-touch-input-mode";

describe("resolveTouchInputMode", () => {
  it("returns explicit inputMode first", () => {
    expect(
      resolveTouchInputMode({
        type: "currency",
        inputMode: "text",
        isCoarsePointer: true,
      }),
    ).toBe("text");
  });

  it("returns undefined on non-coarse pointer", () => {
    expect(
      resolveTouchInputMode({
        type: "currency",
        isCoarsePointer: false,
      }),
    ).toBeUndefined();
  });

  it("maps currency to numeric by default", () => {
    expect(
      resolveTouchInputMode({
        type: "currency",
        isCoarsePointer: true,
      }),
    ).toBe("numeric");
  });

  it("maps currency with decimals to decimal", () => {
    expect(
      resolveTouchInputMode({
        type: "currency",
        isCoarsePointer: true,
        allowDecimalComma: true,
      }),
    ).toBe("decimal");
  });

  it("maps number with integer step to numeric", () => {
    expect(
      resolveTouchInputMode({
        type: "number",
        isCoarsePointer: true,
        step: 1,
      }),
    ).toBe("numeric");
  });

  it("maps number with decimal step to decimal", () => {
    expect(
      resolveTouchInputMode({
        type: "number",
        isCoarsePointer: true,
        step: 0.001,
      }),
    ).toBe("decimal");
  });

  it("maps tel without letters to tel", () => {
    expect(
      resolveTouchInputMode({
        type: "tel",
        isCoarsePointer: true,
      }),
    ).toBe("tel");
  });

  it("leaves text and dni unchanged", () => {
    expect(
      resolveTouchInputMode({
        type: "dni",
        isCoarsePointer: true,
      }),
    ).toBeUndefined();
    expect(
      resolveTouchInputMode({
        type: "text",
        isCoarsePointer: true,
      }),
    ).toBeUndefined();
  });
});

describe("shouldUseTextInputForNumericType", () => {
  it("uses text for number on coarse pointer", () => {
    expect(shouldUseTextInputForNumericType("number", true, false)).toBe(true);
  });

  it("uses text for selectOnFocus number on desktop", () => {
    expect(shouldUseTextInputForNumericType("number", false, true)).toBe(true);
  });

  it("does not force text for plain text fields", () => {
    expect(shouldUseTextInputForNumericType("text", true, false)).toBe(false);
  });
});
