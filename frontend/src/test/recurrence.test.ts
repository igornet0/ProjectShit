import { describe, expect, it } from "vitest";
import { occurrencesBetween } from "@/utils/recurrence";
import type { Recurrence } from "@/types";

const start = "2026-01-31T16:59:59.000Z";
const rule = (frequency: Recurrence["frequency"], interval = 1, until: string | null = null): Recurrence => ({
  frequency,
  interval,
  start,
  until,
});

describe("occurrencesBetween", () => {
  it("keeps month-end for quarterly repeats without drift", () => {
    const dates = occurrencesBetween(rule("quarterly"), new Date(start), new Date("2026-12-31"));
    expect(dates.map((d) => d.toISOString().slice(0, 10))).toEqual([
      "2026-04-30",
      "2026-07-31",
      "2026-10-31",
    ]);
  });

  it("respects interval and until", () => {
    const dates = occurrencesBetween(
      rule("weekly", 2, "2026-03-01T00:00:00.000Z"),
      new Date(start),
      new Date("2026-12-31"),
    );
    expect(dates.map((d) => d.toISOString().slice(0, 10))).toEqual([
      "2026-02-14",
      "2026-02-28",
    ]);
  });
});
