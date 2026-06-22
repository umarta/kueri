// UI helpers for the table/column designers. The actual DDL SQL is generated in
// the Rust backend (src-tauri/src/db/ddl.rs) and invoked via `api.*` — the UI
// never branches on database type to build SQL.
import type { DbKind } from "./types";

export interface ColumnDraft {
  name: string;
  type: string; // a dialect SQL type from typeOptions()
  nullable: boolean;
  primaryKey: boolean;
}

/** Column type choices for the designer dropdown, per dialect. */
export function typeOptions(kind: DbKind): string[] {
  switch (kind) {
    case "mysql":
      return ["INT", "BIGINT", "VARCHAR(255)", "TEXT", "BOOLEAN", "DATETIME", "DATE", "DECIMAL(10,2)", "DOUBLE", "JSON"];
    case "sqlite":
      return ["INTEGER", "TEXT", "REAL", "NUMERIC", "BLOB"];
    case "sqlserver":
      return ["INT", "BIGINT", "NVARCHAR(255)", "NVARCHAR(MAX)", "BIT", "DATETIME2", "DATE", "DECIMAL(10,2)", "FLOAT", "UNIQUEIDENTIFIER"];
    default: // postgres
      return ["serial", "bigserial", "integer", "bigint", "text", "varchar(255)", "boolean", "timestamptz", "date", "numeric", "double precision", "uuid", "jsonb"];
  }
}

/** A sensible first column: an auto-increment-ish primary key per dialect. */
export function defaultIdColumn(kind: DbKind): ColumnDraft {
  const type =
    kind === "mysql" ? "INT" : kind === "sqlite" ? "INTEGER" : kind === "sqlserver" ? "INT" : "serial";
  return { name: "id", type, nullable: false, primaryKey: true };
}

/** Type choices for editing an EXISTING column (drops create-only pseudo-types). */
export function alterTypeOptions(kind: DbKind): string[] {
  return typeOptions(kind).filter((t) => t !== "serial" && t !== "bigserial");
}

/** SQLite has no in-place ALTER COLUMN (type/nullability need a table rebuild). */
export function supportsColumnAlter(kind: DbKind): boolean {
  return kind !== "sqlite";
}
