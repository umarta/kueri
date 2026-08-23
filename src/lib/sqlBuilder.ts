import type { DbKind, QueryResult, RowEdit } from "./types";

function qid(name: string, kind: DbKind): string {
  return kind === "mysql"
    ? "`" + name.replace(/`/g, "``") + "`"
    : '"' + name.replace(/"/g, '""') + '"';
}

function lit(v: unknown): string {
  if (v === null || v === undefined) return "NULL";
  if (typeof v === "number") return String(v);
  if (typeof v === "boolean") return v ? "TRUE" : "FALSE";
  const s = typeof v === "object" ? JSON.stringify(v) : String(v);
  return `'${s.replace(/'/g, "''")}'`;
}

export function buildRowEdits(
  cellEdits: Record<string, string | null>,
  result: QueryResult
): RowEdit[] {
  const byRow: Record<number, RowEdit> = {};
  for (const k of Object.keys(cellEdits)) {
    const [r, c] = k.split(":").map(Number);
    byRow[r] ??= { rowIndex: r, original: result.rows[r], updates: {} };
    byRow[r].updates[result.columns[c]] = cellEdits[k];
  }
  return Object.values(byRow);
}

export function buildUpdateSql(
  rowEdits: RowEdit[],
  columns: string[],
  pkColumns: string[],
  tbl: { schema: string; table: string },
  kind: DbKind
): string {
  const q = (n: string) => qid(n, kind);
  const table = `${q(tbl.schema)}.${q(tbl.table)}`;
  const whereCols = pkColumns.length
    ? pkColumns.filter((c) => columns.includes(c))
    : columns;
  return rowEdits
    .map((re) => {
      const sets = Object.entries(re.updates)
        .map(([col, val]) => `${q(col)} = ${lit(val)}`)
        .join(", ");
      const where = whereCols
        .map((col) => {
          const v = re.original[columns.indexOf(col)];
          return v === null || v === undefined
            ? `${q(col)} IS NULL`
            : `${q(col)} = ${lit(v)}`;
        })
        .join(" AND ");
      return `UPDATE ${table} SET ${sets} WHERE ${where};`;
    })
    .join("\n");
}
