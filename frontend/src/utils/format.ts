/** GitHub linguist colors, used for language dots and project avatars. */
export const LANGUAGE_COLORS: Record<string, string> = {
  rust: "#dea584",
  python: "#3572a5",
  javascript: "#f1e05a",
  typescript: "#3178c6",
  go: "#00add8",
  java: "#b07219",
  cpp: "#f34b7d",
  unknown: "#6b7280",
};

export function languageColor(language: string): string {
  return LANGUAGE_COLORS[language] ?? LANGUAGE_COLORS.unknown;
}

export function shortPath(path: string, max = 56): string {
  const home = path.replace(/^\/(Users|home)\/[^/]+/, "~");
  return home.length > max ? `…${home.slice(-(max - 1))}` : home;
}

const UNITS: Array<[Intl.RelativeTimeFormatUnit, number]> = [
  ["year", 31_536_000],
  ["month", 2_592_000],
  ["week", 604_800],
  ["day", 86_400],
  ["hour", 3_600],
  ["minute", 60],
];

export function formatRelative(iso: string, locale: string): string {
  const seconds = (new Date(iso).getTime() - Date.now()) / 1000;
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  for (const [unit, size] of UNITS) {
    if (Math.abs(seconds) >= size) {
      return rtf.format(Math.round(seconds / size), unit);
    }
  }
  return rtf.format(0, "minute");
}

export function formatDate(
  iso: string,
  locale: string,
  options: Intl.DateTimeFormatOptions = { month: "short", day: "numeric", year: "numeric" },
): string {
  return new Date(iso).toLocaleDateString(locale, options);
}
