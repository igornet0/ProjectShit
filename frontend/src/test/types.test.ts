import { describe, it, expect } from "vitest";
import { LANGUAGE_ICONS } from "@/types";

describe("types", () => {
  it("has language icons for known languages", () => {
    expect(LANGUAGE_ICONS.rust).toBe("🦀");
    expect(LANGUAGE_ICONS.python).toBe("🐍");
  });
});
