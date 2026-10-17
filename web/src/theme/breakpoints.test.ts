import { describe, expect, it } from "vitest";
import { kaiBreakpointAtLeast, resolveKaiBreakpoint } from "./breakpoints";

describe("resolveKaiBreakpoint", () => {
  it("maps widths to Tailwind-aligned names", () => {
    expect(resolveKaiBreakpoint(0)).toBe("xs");
    expect(resolveKaiBreakpoint(639)).toBe("xs");
    expect(resolveKaiBreakpoint(640)).toBe("sm");
    expect(resolveKaiBreakpoint(767)).toBe("sm");
    expect(resolveKaiBreakpoint(768)).toBe("md");
    expect(resolveKaiBreakpoint(1024)).toBe("lg");
    expect(resolveKaiBreakpoint(1280)).toBe("xl");
    expect(resolveKaiBreakpoint(1536)).toBe("2xl");
  });
});

describe("kaiBreakpointAtLeast", () => {
  it("compares breakpoint order", () => {
    expect(kaiBreakpointAtLeast("md", "sm")).toBe(true);
    expect(kaiBreakpointAtLeast("xs", "sm")).toBe(false);
    expect(kaiBreakpointAtLeast("lg", "lg")).toBe(true);
  });
});
