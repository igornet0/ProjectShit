import type { Recurrence } from "@/types";

const MONTHS_PER_STEP: Partial<Record<Recurrence["frequency"], number>> = {
  monthly: 1,
  quarterly: 3,
  yearly: 12,
};
const DAYS_PER_STEP: Partial<Record<Recurrence["frequency"], number>> = {
  daily: 1,
  weekly: 7,
};
const MAX_STEPS = 10_000;

/** Add months in UTC, clamping to the last day of the target month (Jan 31 + 1 → Feb 28). */
function addMonthsClamped(date: Date, months: number): Date {
  const target = new Date(date.getTime());
  const day = target.getUTCDate();
  target.setUTCDate(1);
  target.setUTCMonth(target.getUTCMonth() + months);
  const lastDay = new Date(Date.UTC(target.getUTCFullYear(), target.getUTCMonth() + 1, 0)).getUTCDate();
  target.setUTCDate(Math.min(day, lastDay));
  return target;
}

/** The n-th occurrence counted from the rule's anchor; mirrors the Rust `Recurrence::occurrence`. */
export function occurrence(rule: Recurrence, n: number): Date {
  const start = new Date(rule.start);
  const steps = n * Math.max(1, rule.interval);
  const months = MONTHS_PER_STEP[rule.frequency];
  if (months !== undefined) return addMonthsClamped(start, steps * months);
  const days = DAYS_PER_STEP[rule.frequency] ?? 1;
  return new Date(start.getTime() + steps * days * 86_400_000);
}

/** Occurrences strictly after `after` and up to `end` (and the rule's `until`). */
export function occurrencesBetween(rule: Recurrence, after: Date, end: Date): Date[] {
  const until = rule.until ? new Date(rule.until) : null;
  const result: Date[] = [];
  for (let n = 0; n < MAX_STEPS; n++) {
    const at = occurrence(rule, n);
    if (at > end || (until && at > until)) break;
    if (at > after) result.push(at);
  }
  return result;
}
