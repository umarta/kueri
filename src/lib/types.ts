export type DbKind =
  | "postgres"
  | "mysql"
  | "sqlite"
  | "sqlserver"
  | "redis"
  | "mongodb";

export interface ConnectionConfig {
  id: string;
  name: string;
  kind: DbKind;
  host: string;
  port: number;
  database: string;
  user: string;
  password: string;
  ssl: boolean;
  ssl_mode?: string | null;
  ssl_ca?: string | null;
  ssl_cert?: string | null;
  ssl_key?: string | null;
  file_path?: string | null;
  ssh_enabled?: boolean;
  ssh_host?: string;
  ssh_port?: number;
  ssh_user?: string;
  ssh_key?: string | null;
  // UI-only metadata (ignored by the Rust backend — serde drops unknown fields).
  // `tag` is the environment label, `color` is its status-dot token name,
  // `group` is an optional folder in the connection list.
  tag?: string;
  color?: StatusColor;
  group?: string;
}

// Connection environment colors. Meaning-bearing, never decorative:
// they keep production visually distinct from local at a glance.
export type StatusColor =
  | "local"
  | "staging"
  | "prod"
  | "blue"
  | "orange"
  | "purple"
  | "gray";

export interface SchemaInfo {
  name: string;
}

export interface TableInfo {
  name: string;
  kind: string;
}

export interface ColumnInfo {
  name: string;
  data_type: string;
  nullable: boolean;
  default: string | null;
  enum_values?: string[];
  comment?: string | null;
}

export interface QueryResult {
  columns: string[];
  rows: unknown[][];
  row_count: number;
}

/** One workspace tab. "table" = a sidebar-opened table browser (grid only);
 *  "query" = a SQL editor with its result grid (TablePlus-style separation). */
export interface QueryTab {
  id: string;
  kind: "table" | "query";
  title: string;
  doc: string;
  result: QueryResult | null;
  error: string | null;
  running: boolean;
  view: "data" | "structure";
  selected: { schema: string; table: string } | null;
  editableTable: { schema: string; table: string } | null;
  pkColumns: string[];
  columns: ColumnInfo[];
  filters: FilterCond[];
  filtersOpen: boolean;
  selectedRow: number | null;
  sort: { col: string; dir: "asc" | "desc" } | null;
  offset: number;
  foreignKeys: ForeignKey[];
  /** Result sets from a multi-statement run (empty/1 for a single statement). */
  results: QueryResult[];
  resultIdx: number;
  /** Preview (italic) table tab — a single-click reuses it; double-click pins it. */
  preview: boolean;
}

/** A foreign-key edge: `column` → `ref_schema.ref_table.ref_column`. */
export interface ForeignKey {
  column: string;
  ref_schema: string;
  ref_table: string;
  ref_column: string;
}

/** A running server session/query (Server Monitor). */
export interface ProcessInfo {
  pid: string;
  user: string;
  database: string;
  state: string;
  seconds: number;
  query: string;
}

/** A database role/user. */
export interface RoleInfo {
  name: string;
  attributes: string;
}

/** An installed PostgreSQL client-tools version. */
export interface ClientTool {
  major: string;
  full: string;
  bin: string;
}

/** An index on a table. */
export interface IndexInfo {
  name: string;
  columns: string[];
  unique: boolean;
  method: string;
  predicate: string;
}

/** A single filter condition in the filter bar. */
export interface FilterCond {
  column: string;
  op: FilterOp;
  value: string;
}

export type FilterOp =
  | "="
  | "!="
  | ">"
  | "<"
  | ">="
  | "<="
  | "contains"
  | "starts"
  | "is null"
  | "is not null";

/** A staged row edit emitted by DataGrid on commit. */
export interface RowEdit {
  rowIndex: number;
  original: unknown[];
  updates: Record<string, string | null>; // column name → new value (null = SQL NULL)
}
