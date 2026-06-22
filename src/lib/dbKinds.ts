import type { DbKind } from "./types";

export interface DbKindMeta {
  value: DbKind;
  label: string;
  abbr: string;     // 2-char badge, TablePlus-style
  color: string;    // badge color (brand-ish; identity, not status)
  port: number;     // default port (0 = file-based)
  implemented: boolean;
}

// Single source of truth for the DB type list. Adding a relational driver?
// Add one row here — the picker, the saved list, and the form all read it.
export const DB_KINDS: DbKindMeta[] = [
  { value: "postgres",  label: "PostgreSQL",      abbr: "Pg", color: "#4d8ed6", port: 5432,  implemented: true  },
  { value: "mysql",     label: "MySQL / MariaDB", abbr: "My", color: "#e48e2a", port: 3306,  implemented: true  },
  { value: "sqlite",    label: "SQLite",          abbr: "Sl", color: "#4aa3df", port: 0,     implemented: true  },
  { value: "sqlserver", label: "SQL Server",      abbr: "Ms", color: "#c0413b", port: 1433,  implemented: true  },
  { value: "redis",     label: "Redis",           abbr: "Re", color: "#d23b2e", port: 6379,  implemented: false },
  { value: "mongodb",   label: "MongoDB",         abbr: "Mo", color: "#4aa45a", port: 27017, implemented: false },
];

export function dbKind(value: DbKind): DbKindMeta {
  return DB_KINDS.find((k) => k.value === value) ?? DB_KINDS[0];
}

// Status-dot token (environment), distinct from the brand badge color above.
export const STATUS_COLORS: { value: NonNullable<import("./types").ConnectionConfig["color"]>; label: string; var: string }[] = [
  { value: "local",   label: "Local",      var: "var(--status-local)" },
  { value: "staging", label: "Staging",    var: "var(--status-staging)" },
  { value: "prod",    label: "Production", var: "var(--status-prod)" },
  { value: "blue",    label: "Blue",       var: "var(--status-blue)" },
  { value: "orange",  label: "Orange",     var: "var(--status-orange)" },
  { value: "purple",  label: "Purple",     var: "var(--status-purple)" },
  { value: "gray",    label: "Gray",       var: "var(--status-gray)" },
];

export function statusVar(color?: string): string {
  const found = STATUS_COLORS.find((c) => c.value === color);
  return found ? found.var : "var(--status-gray)";
}
