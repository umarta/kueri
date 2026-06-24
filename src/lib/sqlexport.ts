// Native SQL export — generate CREATE/INSERT text over the live connection,
// no external pg_dump/mysqldump (so it never hits a client/server version
// mismatch). This is what Navicat's "Dump SQL File" does.
import type { DbKind } from "./types";

export function qIdent(kind: DbKind, name: string): string {
  return kind === "mysql"
    ? "`" + name.replace(/`/g, "``") + "`"
    : '"' + name.replace(/"/g, '""') + '"';
}

export function qTable(kind: DbKind, schema: string, table: string): string {
  // SQLite has no schemas; MySQL "schema" is the database (qualifying is optional).
  return kind === "sqlite"
    ? qIdent(kind, table)
    : `${qIdent(kind, schema)}.${qIdent(kind, table)}`;
}

/** A SQL literal for a JSON value coming back from a query result. */
export function sqlLiteral(v: unknown): string {
  if (v === null || v === undefined) return "NULL";
  if (typeof v === "number") return Number.isFinite(v) ? String(v) : "NULL";
  if (typeof v === "boolean") return v ? "TRUE" : "FALSE";
  if (typeof v === "object") return "'" + JSON.stringify(v).replace(/'/g, "''") + "'";
  return "'" + String(v).replace(/'/g, "''") + "'";
}

/** INSERT statements for a result set (empty string when there are no rows). */
export function buildInserts(
  kind: DbKind,
  schema: string,
  table: string,
  columns: string[],
  rows: unknown[][],
): string {
  if (rows.length === 0) return "";
  const cols = columns.map((c) => qIdent(kind, c)).join(", ");
  const t = qTable(kind, schema, table);
  return rows
    .map((r) => `INSERT INTO ${t} (${cols}) VALUES (${r.map(sqlLiteral).join(", ")});`)
    .join("\n");
}
