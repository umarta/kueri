// Shared datetime helpers for the type-aware editors (row detail + grid cell).

// Tz-aware types (Postgres `timestamp with time zone`) are excluded: the picker
// round-trips through local Date components, which would silently shift the
// stored instant. Covers MySQL datetime/timestamp + pg `timestamp without time zone`.
// Covers datetime / timestamp / timestamp with time zone (the original PR
// behaviour). The picker round-trips through local Date components, so a
// tz-aware value is shown/saved in local time.
export const isDateTime = (t: string) => /datetime|timestamp/i.test(t);

/** Plain date columns (no time component). */
export const isDate = (t: string) => /^date$/i.test(t.trim());

/** Parse a DB date/datetime string into a Date (null when empty/unparseable). */
export function toDateValue(value: string | null): Date | null {
  if (!value) return null;
  let s = value.replace(" ", "T");
  // A bare `YYYY-MM-DD` parses as UTC midnight, which can shift a day in
  // negative-offset zones; pin it to local midnight instead.
  if (/^\d{4}-\d{2}-\d{2}$/.test(s)) s += "T00:00:00";
  const date = new Date(s);
  return Number.isNaN(date.getTime()) ? null : date;
}

const pad2 = (n: number) => String(n).padStart(2, "0");

/** Format a Date back into `YYYY-MM-DD HH:MM:SS` for storage. */
export function toDateString(value: Date | null): string | null {
  if (!value) return null;
  return `${value.getFullYear()}-${pad2(value.getMonth() + 1)}-${pad2(value.getDate())} ${pad2(value.getHours())}:${pad2(value.getMinutes())}:${pad2(value.getSeconds())}`;
}

/** Format a Date back into `YYYY-MM-DD` (date-only columns). */
export function toDateOnlyString(value: Date | null): string | null {
  if (!value) return null;
  return `${value.getFullYear()}-${pad2(value.getMonth() + 1)}-${pad2(value.getDate())}`;
}
