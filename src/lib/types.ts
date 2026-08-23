export type DbKind =
  | "postgres"
  | "mysql"
  | "sqlite"
  | "sqlserver"
  | "redis"
  | "mongodb";

// ---------------------------------------------------------------------------
// V2 connection sub-types — mirror the exact JSON shape emitted by the Rust
// backend (serde). Field names and string literals must match Rust's serde
// output exactly; the compiler cannot catch mismatches at runtime.
// ---------------------------------------------------------------------------

/** TLS connection security level. Mirrors `TlsMode` in `src-tauri/src/tls.rs`. */
export type TlsMode =
  | "disable"
  | "allow"
  | "prefer"
  | "require"
  | "verify-ca"
  | "verify-full";

/** TLS configuration block. Mirrors `TlsConfig` in `src-tauri/src/tls.rs`. */
export interface TlsConfig {
  mode: TlsMode;
  ca_path?: string | null;
  cert_path?: string | null;
  key_path?: string | null;
}

/**
 * How a password is sourced. Mirrors `PasswordSource` in
 * `src-tauri/src/secrets/mod.rs` (adjacently-tagged, kebab-case, with
 * explicit renames for `onepassword` and `aws-sm`).
 */
export type PasswordSource =
  | { kind: "plain" }
  | { kind: "keychain" }
  | { kind: "env"; name: string }
  | { kind: "onepassword"; item: string; field: string }
  | { kind: "vault"; path: string; field: string }
  | { kind: "aws-sm"; arn: string; region: string };

/**
 * SSH authentication method. Mirrors `SshAuth` in
 * `src-tauri/src/ssh/profile.rs` (adjacently-tagged, kebab-case).
 */
export type SshAuth =
  | { kind: "password"; source: PasswordSource }
  | { kind: "key-file"; path: string; passphrase?: PasswordSource | null }
  | { kind: "agent" };

/**
 * SSH connection profile. Mirrors `SshProfile` in
 * `src-tauri/src/ssh/profile.rs`.
 */
export interface SshProfile {
  id: string;
  name: string;
  host: string;
  port: number;
  user: string;
  auth: SshAuth;
  jump?: string | null;
}

/**
 * Reference to an SSH profile — either by UUID or inline. Mirrors `SshRef`
 * in `src-tauri/src/ssh/profile.rs` (adjacently-tagged with `tag`/`content`,
 * kebab-case). Verified JSON shape from Task 2:
 *   - `{"kind":"profile","value":"<uuid>"}`
 *   - `{"kind":"inline","value":{...SshProfile}}`
 */
export type SshRef =
  | { kind: "profile"; value: string }
  | { kind: "inline"; value: SshProfile };

/**
 * Query safety guard level. Mirrors `SafetyLevel` in
 * `src-tauri/src/safety.rs` (kebab-case).
 */
export type SafetyLevel =
  | "off"
  | "warn"
  | "confirm-destructive"
  | "confirm-writes"
  | "confirm-ddl"
  | "read-only";

/**
 * V2 connection configuration. Mirrors `ConnectionConfigV2` in
 * `src-tauri/src/db/connect.rs`. Required fields match the Rust struct
 * (no `#[serde(default)]` on id, schema_version, name, kind, password,
 * safety, tags). Optional/nullable fields use `#[serde(default)]` in Rust.
 *
 * UI-only fields (`tag`, `color`, `group`) are not sent to the backend —
 * serde ignores unknown fields. They are preserved here for the transition
 * period (Task 10 consumers still reference them).
 */
export interface ConnectionConfig {
  // --- wire fields (v2) ---
  id: string;
  schema_version: number;
  name: string;
  kind: DbKind;
  host: string;
  port: number;
  database: string;
  user: string;
  password: PasswordSource;
  tls?: TlsConfig | null;
  ssh?: SshRef | null;
  safety: SafetyLevel;
  /** Backend-side tags (migrated from the old `tag` field). */
  tags: string[];
  file_path?: string | null;
  // --- UI-only metadata (ignored by the Rust backend — serde drops unknown fields) ---
  // `tag` is the environment label, `color` is its status-dot token name
  // (the Rust wire also emits `color` as a nullable string; StatusColor is a
  // subset of string so the same field covers both), `group` is an optional
  // folder in the connection list.
  /** Environment color label. Wire type is `string | null`; UI constrains to StatusColor. */
  color?: StatusColor | null;
  tag?: string;
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
  /** The selected object is a view (drives the Structure definition editor). */
  isView: boolean;
  editableTable: { schema: string; table: string } | null;
  pkColumns: string[];
  columns: ColumnInfo[];
  /** Staged cell edits shared by the grid + row-detail panel, keyed `rowIndex:colIndex`. */
  cellEdits: Record<string, string | null>;
  filters: FilterCond[];
  filtersOpen: boolean;
  selectedRow: number | null;
  /** Sort columns in priority order (multi-column sort). */
  sort: { col: string; dir: "asc" | "desc" }[];
  offset: number;
  foreignKeys: ForeignKey[];
  /** Result sets from a multi-statement run (empty/1 for a single statement). */
  results: QueryResult[];
  resultIdx: number;
  /** Preview (italic) table tab — a single-click reuses it; double-click pins it. */
  preview: boolean;
  /** Named parameter values for :name substitution. */
  params: Record<string, string>;
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

// ─── Cross-launch workspace snapshot (KUE-006) ───────────────────────────────

export type PersistedTab =
  | { kind: "query"; id: string; title: string; sql: string }
  | { kind: "table"; id: string; schema: string; table: string };

export interface PersistedWorkspace {
  connection_id: string;
  active_schema: string;
  focused_tab_id: string | null;
  tabs: PersistedTab[];
}

export interface WorkspaceFile {
  schema_version: number;
  last_active_id: string | null;
  workspaces: PersistedWorkspace[];
}
