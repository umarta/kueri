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

/** Tz-aware datetime columns (Postgres `timestamp with time zone`). */
export const isDateTimeTz = (t: string) =>
  /datetime|timestamp/i.test(t) && /with time zone/i.test(t);

/** Common UTC offsets for the timezone selector. */
export const TZ_OFFSETS = [
  "+14:00", "+13:00", "+12:00", "+11:00", "+10:00", "+09:30", "+09:00", "+08:00",
  "+07:00", "+06:30", "+06:00", "+05:45", "+05:30", "+05:00", "+04:30", "+04:00",
  "+03:30", "+03:00", "+02:00", "+01:00", "+00:00", "-01:00", "-02:00", "-03:00",
  "-03:30", "-04:00", "-05:00", "-06:00", "-07:00", "-08:00", "-09:00", "-10:00",
  "-11:00", "-12:00",
];

/** This machine's current UTC offset, e.g. "+07:00". */
export function localOffset(): string {
  const m = -new Date().getTimezoneOffset();
  const sign = m >= 0 ? "+" : "-";
  return `${sign}${pad2(Math.floor(Math.abs(m) / 60))}:${pad2(Math.abs(m) % 60)}`;
}

const normOffset = (o: string) =>
  o === "Z" ? "+00:00" : /^[+-]\d{4}$/.test(o) ? `${o.slice(0, 3)}:${o.slice(3)}` : o;

/** Split a stored tz timestamp into its wall-clock + offset (WYSIWYG editing). */
export function splitTz(value: string | null): { wall: string; offset: string } {
  if (!value) return { wall: "", offset: localOffset() };
  let s = value.trim();
  let offset = localOffset();
  const m = s.match(/(Z|[+-]\d{2}:?\d{2})$/);
  if (m) {
    offset = normOffset(m[1]);
    s = s.slice(0, m.index);
  }
  s = s.replace("T", " ").replace(/\.\d+$/, "").trim();
  return { wall: s, offset };
}

/** Recombine a wall-clock + offset into a tz literal the DB accepts. */
export const combineTz = (wall: string, offset: string) => (wall ? `${wall}${offset}` : "");

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
