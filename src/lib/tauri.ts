import { invoke } from "@tauri-apps/api/core";
import type {
  ConnectionConfig,
  SchemaInfo,
  TableInfo,
  ColumnInfo,
  QueryResult,
  ForeignKey,
  IndexInfo,
  ProcessInfo,
  RoleInfo,
  ClientTool,
} from "./types";
import type { ColumnDraft } from "./ddl";

// One typed surface over all Rust commands. Add new commands here.
export const api = {
  connect: (config: ConnectionConfig) => invoke<string>("connect", { config }),
  disconnect: (id: string) => invoke<void>("disconnect", { id }),
  listSchemas: (id: string) => invoke<SchemaInfo[]>("list_schemas", { id }),
  listTables: (id: string, schema: string) =>
    invoke<TableInfo[]>("list_tables", { id, schema }),
  listColumns: (id: string, schema: string, table: string) =>
    invoke<ColumnInfo[]>("list_columns", { id, schema, table }),
  tableDdl: (id: string, schema: string, table: string) =>
    invoke<string>("table_ddl", { id, schema, table }),
  foreignKeys: (id: string, schema: string, table: string) =>
    invoke<ForeignKey[]>("foreign_keys", { id, schema, table }),
  listIndexes: (id: string, schema: string, table: string) =>
    invoke<IndexInfo[]>("list_indexes", { id, schema, table }),
  createIndex: (id: string, schema: string, table: string, name: string, columns: string[], unique: boolean) =>
    invoke<void>("create_index", { id, schema, table, name, columns, unique }),
  dropIndex: (id: string, schema: string, table: string, name: string) =>
    invoke<void>("drop_index", { id, schema, table, name }),
  addForeignKey: (id: string, schema: string, table: string, column: string, refTable: string, refColumn: string, name: string, validate: boolean) =>
    invoke<void>("add_foreign_key", { id, schema, table, column, refTable, refColumn, name, validate }),
  primaryKeys: (id: string, schema: string, table: string) =>
    invoke<string[]>("primary_keys", { id, schema, table }),
  executeQuery: (id: string, sql: string, queryId: string) =>
    invoke<QueryResult>("execute_query", { id, sql, queryId }),
  cancelQuery: (queryId: string) => invoke<void>("cancel_query", { queryId }),
  beginTxn: (id: string) => invoke<void>("begin_txn", { id }),
  commitTxn: (id: string) => invoke<void>("commit_txn", { id }),
  rollbackTxn: (id: string) => invoke<void>("rollback_txn", { id }),
  listProcesses: (id: string) => invoke<ProcessInfo[]>("list_processes", { id }),
  killProcess: (id: string, pid: string) => invoke<void>("kill_process", { id, pid }),
  listRoles: (id: string) => invoke<RoleInfo[]>("list_roles", { id }),
  createSchema: (id: string, name: string) => invoke<void>("create_schema", { id, name }),
  dropSchema: (id: string, name: string) => invoke<void>("drop_schema", { id, name }),

  // DDL — database-agnostic; the Rust driver renders the right SQL per dialect.
  createTable: (id: string, schema: string, name: string, columns: ColumnDraft[]) =>
    invoke<void>("create_table", { id, schema, name, columns }),
  dropTable: (id: string, schema: string, table: string) =>
    invoke<void>("drop_table", { id, schema, table }),
  renameTable: (id: string, schema: string, oldName: string, newName: string) =>
    invoke<void>("rename_table", { id, schema, old: oldName, new: newName }),
  truncateTable: (id: string, schema: string, table: string) =>
    invoke<void>("truncate_table", { id, schema, table }),
  duplicateTable: (id: string, schema: string, src: string, dst: string) =>
    invoke<void>("duplicate_table", { id, schema, src, dst }),
  addColumn: (id: string, schema: string, table: string, column: ColumnDraft) =>
    invoke<void>("add_column", { id, schema, table, column }),
  dropColumn: (id: string, schema: string, table: string, column: string) =>
    invoke<void>("drop_column", { id, schema, table, column }),
  renameColumn: (id: string, schema: string, table: string, oldName: string, newName: string) =>
    invoke<void>("rename_column", { id, schema, table, old: oldName, new: newName }),
  changeColumnType: (id: string, schema: string, table: string, column: string, newType: string, notNull: boolean) =>
    invoke<void>("change_column_type", { id, schema, table, column, newType, notNull }),
  setColumnNullable: (id: string, schema: string, table: string, column: string, currentType: string, notNull: boolean) =>
    invoke<void>("set_column_nullable", { id, schema, table, column, currentType, notNull }),

  // PostgreSQL backup & restore (shells out to pg_dump / pg_restore / psql).
  pgExport: (cfg: ConnectionConfig, path: string, format: string, contents: string, tools: string) =>
    invoke<string>("pg_export", { cfg, path, format, contents, tools }),
  pgImport: (cfg: ConnectionConfig, path: string, tools: string) =>
    invoke<string>("pg_import", { cfg, path, tools }),
  detectClients: () => invoke<ClientTool[]>("detect_clients"),
  installPgClient: (major: string) => invoke<string>("install_pg_client", { major }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),

  // Write a text file (CSV/JSON export; path comes from a save dialog).
  writeTextFile: (path: string, content: string) =>
    invoke<void>("write_text_file", { path, content }),
  readTextFile: (path: string) => invoke<string>("read_text_file", { path }),

  // Persistence (connections file) + OS keychain (passwords).
  loadConnections: () => invoke<ConnectionConfig[]>("load_connections"),
  saveConnections: (connections: Omit<ConnectionConfig, "password">[]) =>
    invoke<void>("save_connections", { connections }),
  secretSet: (id: string, password: string) =>
    invoke<void>("secret_set", { id, password }),
  secretGet: (id: string) => invoke<string | null>("secret_get", { id }),
  secretDelete: (id: string) => invoke<void>("secret_delete", { id }),
};
