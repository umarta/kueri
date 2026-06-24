// Shared datetime helpers for the type-aware editors (row detail + grid cell).

// Tz-aware types (Postgres `timestamp with time zone`) are excluded: the picker
// round-trips through local Date components, which would silently shift the
// stored instant. Covers MySQL datetime/timestamp + pg `timestamp without time zone`.
export const isDateTime = (t: string) =>
  /datetime|timestamp/i.test(t) && !/with time zone/i.test(t);

/** Parse a DB datetime string into a Date (null when empty/unparseable). */
export function toDateValue(value: string | null): Date | null {
  if (!value) return null;
  const date = new Date(value.replace(" ", "T"));
  return Number.isNaN(date.getTime()) ? null : date;
}

const pad2 = (n: number) => String(n).padStart(2, "0");

/** Format a Date back into `YYYY-MM-DD HH:MM:SS` for storage. */
export function toDateString(value: Date | null): string | null {
  if (!value) return null;
  return `${value.getFullYear()}-${pad2(value.getMonth() + 1)}-${pad2(value.getDate())} ${pad2(value.getHours())}:${pad2(value.getMinutes())}:${pad2(value.getSeconds())}`;
}
