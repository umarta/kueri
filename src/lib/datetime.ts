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

/** UTC offsets for the timezone selector, with human city/region hints. */
export const TZ_OFFSETS: { offset: string; label: string }[] = [
  { offset: "+14:00", label: "Kiritimati" },
  { offset: "+13:00", label: "Apia · Tonga" },
  { offset: "+12:00", label: "Auckland · Fiji" },
  { offset: "+11:00", label: "Solomon Is. · Noumea" },
  { offset: "+10:00", label: "Sydney · Brisbane" },
  { offset: "+09:30", label: "Adelaide" },
  { offset: "+09:00", label: "Tokyo · Seoul · WIT" },
  { offset: "+08:00", label: "Singapore · Beijing · WITA" },
  { offset: "+07:00", label: "Jakarta (WIB) · Bangkok" },
  { offset: "+06:30", label: "Yangon" },
  { offset: "+06:00", label: "Dhaka · Almaty" },
  { offset: "+05:45", label: "Kathmandu" },
  { offset: "+05:30", label: "India · Colombo" },
  { offset: "+05:00", label: "Karachi · Tashkent" },
  { offset: "+04:30", label: "Kabul" },
  { offset: "+04:00", label: "Dubai · Baku" },
  { offset: "+03:30", label: "Tehran" },
  { offset: "+03:00", label: "Moscow · Istanbul · Nairobi" },
  { offset: "+02:00", label: "Cairo · Athens · Johannesburg" },
  { offset: "+01:00", label: "Berlin · Paris · Lagos" },
  { offset: "+00:00", label: "UTC · London · Accra" },
  { offset: "-01:00", label: "Azores · Cape Verde" },
  { offset: "-02:00", label: "Fernando de Noronha" },
  { offset: "-03:00", label: "São Paulo · Buenos Aires" },
  { offset: "-03:30", label: "Newfoundland" },
  { offset: "-04:00", label: "Santiago · Caracas" },
  { offset: "-05:00", label: "New York · Toronto · Lima" },
  { offset: "-06:00", label: "Chicago · Mexico City" },
  { offset: "-07:00", label: "Denver · Phoenix" },
  { offset: "-08:00", label: "Los Angeles · Vancouver" },
  { offset: "-09:00", label: "Anchorage" },
  { offset: "-10:00", label: "Honolulu" },
  { offset: "-11:00", label: "Midway" },
  { offset: "-12:00", label: "Baker Island" },
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
