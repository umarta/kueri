<script lang="ts">
  import { tick, onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { open as openFileDialog, ask, save } from "@tauri-apps/plugin-dialog";
  import Welcome from "./components/Welcome.svelte";
  import Toolbar from "./components/Toolbar.svelte";
  import WorkspaceRail from "./components/WorkspaceRail.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import QueryTabs from "./components/QueryTabs.svelte";
  import QueryEditor from "./components/QueryEditor.svelte";
  import DataGrid from "./components/DataGrid.svelte";
  import StructureView from "./components/StructureView.svelte";
  import FilterBar from "./components/FilterBar.svelte";
  import RowDetail from "./components/RowDetail.svelte";
  import CommandPalette from "./components/CommandPalette.svelte";
  import LogPanel from "./components/LogPanel.svelte";
  import Settings from "./components/Settings.svelte";
  import ImportDialog from "./components/ImportDialog.svelte";
  import ExportDialog from "./components/ExportDialog.svelte";
  import { settings } from "./lib/stores/settings";
  import {
    activeConnectionId, activeConnection, schemaCatalog, catalogColumns, workspaces, activeSchema,
    readOnly, isReadStatement, shouldStartReadOnly,
  } from "./lib/stores/connection";
  import { api } from "./lib/tauri";
  import { logSql } from "./lib/stores/log";
  import type { ConnectionConfig, RowEdit, QueryTab } from "./lib/types";

  let sidebarOpen = true;
  let sidebar: Sidebar;
  let grid: DataGrid;
  let editor: QueryEditor;
  let paletteOpen = false;
  let logOpen = false;
  let detailOpen = false;
  let inserting = false;
  let insertInitial: Record<string, string | null> | null = null;
  let insertNonce = 0;
  let settingsOpen = false;
  let exportOpen = false;
  let csvImportOpen = false;
  let toast: { ok: boolean; msg: string } | null = null;
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  function showToast(ok: boolean, msg: string) {
    toast = { ok, msg };
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = null), ok ? 6000 : 12000);
  }

  function openExport() {
    if ($activeConnection?.kind !== "postgres") {
      showToast(false, "Export & restore currently supports PostgreSQL only.");
      return;
    }
    exportOpen = true;
  }

  async function runImport() {
    const cfg = $activeConnection;
    if (!cfg) return;
    if (cfg.kind !== "postgres") {
      showToast(false, "Export & restore currently supports PostgreSQL only.");
      return;
    }
    const picked = await openFileDialog({
      filters: [{ name: "SQL / dump", extensions: ["sql", "dump", "backup", "pgdump"] }],
    });
    const path = Array.isArray(picked) ? picked[0] : picked;
    if (!path) return;
    showToast(true, "Importing…");
    try {
      const msg = await api.pgImport(cfg, path);
      showToast(true, msg);
      await refresh();
    } catch (e) {
      showToast(false, (e as { message?: string })?.message ?? String(e));
    }
  }

  // Cancel the active tab's in-flight query (the backend aborts the task).
  function cancelActive() {
    if (tab.running) api.cancelQuery(tab.id).catch(() => {});
  }

  // ── CSV import ────────────────────────────────────────────────────────────
  function openCsvImport() {
    if (tab.kind !== "table" || !tab.selected || tab.columns.length === 0) {
      showToast(false, "Open a table first to import a CSV into it.");
      return;
    }
    csvImportOpen = true;
  }
  async function runCsvImport(e: CustomEvent<{ columns: string[]; rows: string[][] }>) {
    csvImportOpen = false;
    const t = tab;
    if (t.kind !== "table" || !t.selected || !$activeConnectionId) return;
    if ($readOnly) { showToast(false, blockedMsg); return; }
    const { schema, table } = t.selected;
    const { columns: cols, rows } = e.detail;
    if (!cols.length || !rows.length) return;
    const into = qtable(schema, table);
    const collist = cols.map(qid).join(", ");
    const BATCH = 200;
    let ok = 0;
    let failErr = "";
    t.running = true; sync();
    for (let i = 0; i < rows.length; i += BATCH) {
      const chunk = rows.slice(i, i + BATCH);
      const values = chunk.map((r) => `(${r.map((v) => (v === "" ? "NULL" : lit(v))).join(", ")})`).join(", ");
      const sql = `INSERT INTO ${into} (${collist}) VALUES ${values};`;
      try {
        const s = performance.now();
        await api.executeQuery($activeConnectionId, sql, t.id);
        logSql(`-- import ${chunk.length} rows into ${schema}.${table}`, { ms: Math.round(performance.now() - s) });
        ok += chunk.length;
      } catch (err) {
        failErr = (err as { message?: string })?.message ?? String(err);
        break;
      }
    }
    t.running = false; sync();
    if (failErr) showToast(false, `Imported ${ok} rows, then failed: ${failErr}`);
    else showToast(true, `Imported ${ok} row${ok === 1 ? "" : "s"} into ${schema}.${table}.`);
    await browseTable(t, schema, table);
  }

  // Load a statement from the history panel into a fresh query tab.
  function openSqlTab(sql: string, title: string) {
    const t = blankQueryTab();
    t.doc = sql;
    t.title = title;
    tabs = [...tabs, t];
    activeId = t.id;
  }
  function openHistoryQuery(sql: string) {
    openSqlTab(sql, "History");
  }

  // ── Generate SQL from the open table (#32) ───────────────────────────────────
  async function generateSql(kind: "select" | "insert" | "update" | "create") {
    if (tab.kind !== "table" || !tab.selected || tab.columns.length === 0) {
      showToast(false, "Open a table first to generate SQL.");
      return;
    }
    const { schema, table } = tab.selected;
    const t = qtable(schema, table);
    const cols = tab.columns.map((c) => c.name);
    let sql = "";
    if (kind === "select") {
      sql = `SELECT ${cols.map(qid).join(", ")}\nFROM ${t}\nLIMIT 100;`;
    } else if (kind === "insert") {
      sql = `INSERT INTO ${t} (${cols.map(qid).join(", ")})\nVALUES (${cols.map(() => "NULL").join(", ")});`;
    } else if (kind === "update") {
      const pk = tab.pkColumns.length ? tab.pkColumns : [cols[0]];
      const sets = cols.filter((c) => !pk.includes(c)).map((c) => `${qid(c)} = NULL`).join(",\n    ");
      const where = pk.map((c) => `${qid(c)} = NULL`).join(" AND ");
      sql = `UPDATE ${t}\nSET ${sets}\nWHERE ${where};`;
    } else {
      try {
        sql = await api.tableDdl($activeConnectionId!, schema, table);
      } catch (e) {
        showToast(false, (e as { message?: string })?.message ?? String(e));
        return;
      }
    }
    openSqlTab(sql, `${table} ${kind}`);
  }

  // ── EXPLAIN the current query (#34) ──────────────────────────────────────────
  function explainQuery() {
    if (tab.kind !== "query") return;
    const sql = tab.doc.trim().replace(/;+\s*$/, "");
    if (!sql) return;
    const prefix = $activeConnection?.kind === "sqlite" ? "EXPLAIN QUERY PLAN " : "EXPLAIN ";
    runSql(tab, prefix + sql);
  }

  // ── Query tabs ──────────────────────────────────────────────────────────────
  let seq = 1;
  function blankQueryTab(): QueryTab {
    return {
      id: crypto.randomUUID(), kind: "query", title: `Query ${seq++}`, doc: "SELECT now();",
      result: null, error: null, running: false, view: "data",
      selected: null, editableTable: null, pkColumns: [], columns: [],
      filters: [], filtersOpen: false, selectedRow: null, sort: null, offset: 0, foreignKeys: [], results: [], resultIdx: 0,
    };
  }
  function tableTab(schema: string, table: string): QueryTab {
    return {
      id: crypto.randomUUID(), kind: "table", title: table, doc: "",
      result: null, error: null, running: false, view: "data",
      selected: { schema, table }, editableTable: null, pkColumns: [], columns: [],
      filters: [], filtersOpen: false, selectedRow: null, sort: null, offset: 0, foreignKeys: [], results: [], resultIdx: 0,
    };
  }
  let tabs: QueryTab[] = [blankQueryTab()];
  let activeId = tabs[0].id;
  $: tab = tabs.find((t) => t.id === activeId) ?? tabs[0];
  // Editable when the tab resolves to a single updatable table — always true for a
  // table-browse tab, and for a query tab whose SQL is a simple `SELECT * FROM <table>`.
  $: editing = !!tab.editableTable && !tab.running;
  const sync = () => (tabs = tabs); // commit mutations of a tab back to the array

  function newTab() {
    const t = blankQueryTab();
    tabs = [...tabs, t];
    activeId = t.id;
  }
  function closeTab(id: string) {
    if (tabs.length === 1) return;
    const idx = tabs.findIndex((t) => t.id === id);
    tabs = tabs.filter((t) => t.id !== id);
    if (activeId === id) activeId = tabs[Math.max(0, idx - 1)].id;
  }

  // ── Query execution (always scoped to a specific tab `t`) ────────────────────
  async function exec(t: QueryTab, sql: string) {
    if (!$activeConnectionId) return;
    t.running = true; t.error = null; sync();
    const start = performance.now();
    try {
      t.result = await api.executeQuery($activeConnectionId, sql, t.id);
      logSql(sql, { ms: Math.round(performance.now() - start) });
    } catch (e) {
      t.error = String(e); t.result = null;
      logSql(sql, { ms: Math.round(performance.now() - start), error: String(e) });
    } finally {
      t.running = false; sync();
    }
  }

  // Editor path: detect whether the query maps to one updatable table so the
  // result (grid + row detail) can be edited; otherwise it stays read-only.
  const blockedMsg = "Read-only mode is on for this connection — toggle the lock in the toolbar to allow writes.";

  // Split a script into statements, respecting quotes, comments and $tag$ blocks.
  function splitStatements(sql: string): string[] {
    const out: string[] = [];
    let cur = "";
    let i = 0;
    const n = sql.length;
    let dollar: string | null = null;
    while (i < n) {
      const c = sql[i];
      if (dollar) {
        if (sql.startsWith(dollar, i)) { cur += dollar; i += dollar.length; dollar = null; continue; }
        cur += c; i++; continue;
      }
      if (c === "'" || c === '"' || c === "`") {
        const q = c; cur += c; i++;
        while (i < n) {
          cur += sql[i];
          if (sql[i] === q) { if (sql[i + 1] === q) { cur += sql[i + 1]; i += 2; continue; } i++; break; }
          i++;
        }
        continue;
      }
      if (c === "-" && sql[i + 1] === "-") { while (i < n && sql[i] !== "\n") { cur += sql[i]; i++; } continue; }
      if (c === "/" && sql[i + 1] === "*") { cur += "/*"; i += 2; while (i < n && !(sql[i] === "*" && sql[i + 1] === "/")) { cur += sql[i]; i++; } cur += "*/"; i += 2; continue; }
      if (c === "$") { const m = /^\$[A-Za-z0-9_]*\$/.exec(sql.slice(i)); if (m) { dollar = m[0]; cur += m[0]; i += m[0].length; continue; } }
      if (c === ";") { if (cur.trim()) out.push(cur.trim()); cur = ""; i++; continue; }
      cur += c; i++;
    }
    if (cur.trim()) out.push(cur.trim());
    return out;
  }

  function selectResult(t: QueryTab, idx: number) {
    if (idx < 0 || idx >= t.results.length) return;
    t.resultIdx = idx;
    t.result = t.results[idx];
    sync();
  }

  async function runSql(t: QueryTab, sql: string) {
    const stmts = splitStatements(sql);
    if (!stmts.length) return;
    if ($readOnly && stmts.some((s) => !isReadStatement(s))) { showToast(false, blockedMsg); return; }
    t.editableTable = null; t.pkColumns = []; t.columns = []; t.results = []; t.resultIdx = 0; sync();
    // Single statement: keep the editable single-table path.
    if (stmts.length === 1) {
      await exec(t, stmts[0]);
      if (t.result) await resolveEditable(t);
      return;
    }
    // Multiple statements: run in order, collect each result, stop on first error.
    t.running = true; t.error = null; t.result = null; sync();
    const collected: import("./lib/types").QueryResult[] = [];
    for (let idx = 0; idx < stmts.length; idx++) {
      const s = stmts[idx];
      const start = performance.now();
      try {
        const r = await api.executeQuery($activeConnectionId!, s, t.id);
        collected.push(r);
        logSql(s, { ms: Math.round(performance.now() - start) });
      } catch (e) {
        t.error = `Statement ${idx + 1} of ${stmts.length} failed: ${(e as { message?: string })?.message ?? String(e)}`;
        logSql(s, { ms: Math.round(performance.now() - start), error: String(e) });
        break;
      }
    }
    t.results = collected;
    t.resultIdx = Math.max(0, collected.length - 1);
    t.result = collected[t.resultIdx] ?? null;
    t.running = false;
    sync();
  }

  /** A query is editable only if it's `SELECT * FROM <one table>` (no joins,
   *  aggregates, DISTINCT, GROUP BY, UNION). Returns the resolved table or null. */
  function detectEditableTable(sql: string): { schema: string; table: string } | null {
    let s = sql.replace(/\/\*[\s\S]*?\*\//g, " ").replace(/--[^\n]*/g, " ").trim().replace(/;+\s*$/, "");
    const lower = s.toLowerCase();
    if (!lower.startsWith("select")) return null;
    if (/\bjoin\b|\bgroup\s+by\b|\bunion\b|\bdistinct\b|\bhaving\b|\bwindow\b/.test(lower)) return null;
    const m = /^select\s+([\s\S]*?)\sfrom\s+([\s\S]+)$/i.exec(s);
    if (!m) return null;
    // Require SELECT * (or alias.*) so every result column is a real, updatable column.
    const sel = m[1].trim();
    if (sel !== "*" && !/^[\w"`]+\.\*$/.test(sel)) return null;
    let from = m[2].trim().split(/\b(where|order\s+by|limit|group\s+by|having|offset|fetch|for)\b/i)[0].trim();
    if (from.includes(",")) return null; // multiple tables
    const ref = from.split(/\s+/)[0]; // table token (drop any alias)
    const parts = ref.split(".").map((p) => p.replace(/^["`]|["`]$/g, ""));
    if (parts.length === 2 && parts[0] && parts[1]) return { schema: parts[0], table: parts[1] };
    if (parts.length === 1 && parts[0]) return { schema: $activeSchema || "public", table: parts[0] };
    return null;
  }

  async function resolveEditable(t: QueryTab) {
    const det = detectEditableTable(t.doc);
    if (!det || !$activeConnectionId) {
      t.editableTable = null; t.pkColumns = []; t.columns = []; sync();
      return;
    }
    try {
      t.columns = await api.listColumns($activeConnectionId, det.schema, det.table);
      t.pkColumns = await api.primaryKeys($activeConnectionId, det.schema, det.table).catch(() => []);
      t.editableTable = { schema: det.schema, table: det.table };
    } catch {
      t.editableTable = null; t.pkColumns = []; t.columns = [];
    }
    sync();
  }

  // Build a WHERE clause from the tab's filter conditions (Postgres-style quoting,
  // matching the browse SELECT below).
  // Identifier quoting per dialect — MySQL uses backticks, the rest double quotes.
  function qid(name: string): string {
    return $activeConnection?.kind === "mysql"
      ? "`" + name.replace(/`/g, "``") + "`"
      : '"' + name.replace(/"/g, '""') + '"';
  }
  function qtable(schema: string, table: string): string {
    return `${qid(schema)}.${qid(table)}`;
  }

  function buildWhere(t: QueryTab): string {
    if (!t.filters.length) return "";
    const pg = $activeConnection?.kind === "postgres";
    const parts = t.filters.map((f) => {
      const col = qid(f.column);
      switch (f.op) {
        case "is null": return `${col} IS NULL`;
        case "is not null": return `${col} IS NOT NULL`;
        // Postgres needs a cast + ILIKE; MySQL/SQLite LIKE is already case-insensitive.
        case "contains": return pg ? `${col}::text ILIKE ${lit("%" + f.value + "%")}` : `${col} LIKE ${lit("%" + f.value + "%")}`;
        case "starts": return pg ? `${col}::text ILIKE ${lit(f.value + "%")}` : `${col} LIKE ${lit(f.value + "%")}`;
        case "!=": return `${col} <> ${lit(f.value)}`;
        default: return `${col} ${f.op} ${lit(f.value)}`;
      }
    });
    return ` WHERE ${parts.join(" AND ")}`;
  }

  // Table browse: a plain SELECT * — safe to edit and refresh in place.
  async function browseTable(t: QueryTab, schema: string, table: string) {
    t.selected = { schema, table };
    t.title = table;
    t.selectedRow = null;
    t.results = [];
    const order = t.sort ? ` ORDER BY ${qid(t.sort.col)} ${t.sort.dir === "desc" ? "DESC" : "ASC"}` : "";
    const off = t.offset > 0 ? ` OFFSET ${t.offset}` : "";
    t.doc = `SELECT * FROM ${qtable(schema, table)}${buildWhere(t)}${order} LIMIT ${$settings.rowLimit}${off};`;
    sync();
    await exec(t, t.doc);
    t.editableTable = t.result ? { schema, table } : null;
    if (t.result) {
      catalogColumns(table, t.result.columns);
      try {
        t.pkColumns = await api.primaryKeys($activeConnectionId!, schema, table);
      } catch {
        t.pkColumns = [];
      }
      t.foreignKeys = await api.foreignKeys($activeConnectionId!, schema, table).catch(() => []);
    }
    // Load column types too — powers the Structure tab AND the row-detail panel.
    await loadColumns(t);
    sync();
  }

  function onSelectTable(e: CustomEvent<{ schema: string; table: string }>) {
    openTable(e.detail.schema, e.detail.table);
  }

  function openTable(schema: string, table: string) {
    // Focus an existing table tab for this table; else reuse the active table tab;
    // else open a new table tab. SQL query tabs are never hijacked.
    const existing = tabs.find(
      (x) => x.kind === "table" && x.selected?.schema === schema && x.selected?.table === table,
    );
    if (existing) {
      activeId = existing.id;
      return;
    }
    let t: QueryTab;
    if (tab.kind === "table") {
      t = tab;
      t.filters = [];
      t.sort = null;
      t.offset = 0;
    } else {
      t = tableTab(schema, table);
      tabs = [...tabs, t];
    }
    activeId = t.id;
    t.selectedRow = null;
    sync();
    return browseTable(t, schema, table);
  }

  // ── Foreign-key navigation ───────────────────────────────────────────────────
  function openTableFiltered(schema: string, table: string, column: string, value: string) {
    const t = tableTab(schema, table);
    t.filters = [{ column, op: "=", value }];
    tabs = [...tabs, t];
    activeId = t.id;
    sync();
    browseTable(t, schema, table);
  }
  function followFk(e: CustomEvent<{ column: string; value: string }>) {
    const fk = tab.foreignKeys.find((f) => f.column === e.detail.column);
    if (!fk || !fk.ref_column) return;
    openTableFiltered(fk.ref_schema || $activeSchema || "public", fk.ref_table, fk.ref_column, e.detail.value);
  }

  // ── Filters ───────────────────────────────────────────────────────────────
  function applyFilters(e: CustomEvent<import("./lib/types").FilterCond[]>) {
    if (!tab.selected) return;
    tab.filters = e.detail;
    tab.offset = 0;
    sync();
    browseTable(tab, tab.selected.schema, tab.selected.table);
  }
  function clearFilters() {
    if (!tab.selected) return;
    tab.filters = [];
    tab.offset = 0;
    sync();
    browseTable(tab, tab.selected.schema, tab.selected.table);
  }

  // ── Pagination (table tabs) ─────────────────────────────────────────────────
  function pagePrev() {
    if (tab.kind !== "table" || !tab.selected || tab.offset === 0) return;
    tab.offset = Math.max(0, tab.offset - $settings.rowLimit);
    sync();
    browseTable(tab, tab.selected.schema, tab.selected.table);
  }
  function pageNext() {
    if (tab.kind !== "table" || !tab.selected) return;
    if ((tab.result?.rows.length ?? 0) < $settings.rowLimit) return; // last page
    tab.offset += $settings.rowLimit;
    sync();
    browseTable(tab, tab.selected.schema, tab.selected.table);
  }

  // ── Sort (table tabs) ───────────────────────────────────────────────────────
  function toggleSort(col: string) {
    if (tab.kind !== "table" || !tab.selected) return;
    const cur = tab.sort;
    if (!cur || cur.col !== col) tab.sort = { col, dir: "asc" };
    else if (cur.dir === "asc") tab.sort = { col, dir: "desc" };
    else tab.sort = null;
    tab.offset = 0;
    sync();
    browseTable(tab, tab.selected.schema, tab.selected.table);
  }

  async function loadColumns(t: QueryTab) {
    if (!$activeConnectionId || !t.selected) return;
    t.columns = await api.listColumns($activeConnectionId, t.selected.schema, t.selected.table);
    sync();
  }

  async function setView(v: "data" | "structure") {
    tab.view = v; sync();
    if (v === "structure" && tab.selected && tab.columns.length === 0) await loadColumns(tab);
  }

  // SQL literal — quoted literals are "unknown"-typed and coerced to the column type.
  function lit(v: unknown): string {
    if (v === null || v === undefined) return "NULL";
    if (typeof v === "number") return String(v);
    if (typeof v === "boolean") return v ? "TRUE" : "FALSE";
    const s = typeof v === "object" ? JSON.stringify(v) : String(v);
    return `'${s.replace(/'/g, "''")}'`;
  }

  async function commitEdits(e: CustomEvent<RowEdit[]>) {
    if ($readOnly) { showToast(false, blockedMsg); return; }
    const t = tab;
    const tbl = t.editableTable;
    if (!tbl || !$activeConnectionId || !t.result) return;
    const cols = t.result.columns;
    const whereCols = t.pkColumns.length ? t.pkColumns.filter((c) => cols.includes(c)) : cols;
    t.running = true; t.error = null; sync();
    try {
      for (const ch of e.detail) {
        const sets = Object.entries(ch.updates)
          .map(([col, val]) => `${qid(col)} = ${lit(val)}`)
          .join(", ");
        const where = whereCols
          .map((col) => {
            const v = ch.original[cols.indexOf(col)];
            return v === null || v === undefined ? `${qid(col)} IS NULL` : `${qid(col)} = ${lit(v)}`;
          })
          .join(" AND ");
        const upd = `UPDATE ${qtable(tbl.schema, tbl.table)} SET ${sets} WHERE ${where};`;
        const us = performance.now();
        await api.executeQuery($activeConnectionId, upd, t.id);
        logSql(upd, { ms: Math.round(performance.now() - us) });
      }
    } catch (err) {
      t.error = String(err); t.running = false; sync(); return;
    }
    t.running = false; sync();
    // Refresh from source of truth — re-browse for a table tab, or re-run the
    // original query (so a query tab keeps its WHERE/ORDER/LIMIT) for a query tab.
    if (t.kind === "table") {
      await browseTable(t, tbl.schema, tbl.table);
    } else {
      await exec(t, t.doc);
      if (t.result) await resolveEditable(t);
    }
  }

  // ── Export result (CSV / JSON) ──────────────────────────────────────────────
  // Build CSV / JSON / SQL-INSERT text for a set of rows.
  function buildExport(format: string, columns: string[], rows: unknown[][], sqlTable: string): string {
    if (format === "json") {
      return JSON.stringify(rows.map((row) => Object.fromEntries(columns.map((c, i) => [c, row[i]]))), null, 2);
    }
    if (format === "sql") {
      const cols = columns.map(qid).join(", ");
      return rows
        .map((row) => `INSERT INTO ${sqlTable} (${cols}) VALUES (${row.map((v) => (v === null || v === undefined ? "NULL" : lit(typeof v === "object" ? JSON.stringify(v) : v))).join(", ")});`)
        .join("\n");
    }
    const esc = (v: unknown) => {
      const s = v === null || v === undefined ? "" : typeof v === "object" ? JSON.stringify(v) : String(v);
      return /[",\n\r]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
    };
    return [columns.map(esc).join(","), ...rows.map((row) => row.map(esc).join(","))].join("\n");
  }

  // Export the current result (whole=false) or the whole table (whole=true).
  // Format follows the chosen file extension (.csv / .json / .sql).
  async function doExport(whole: boolean) {
    if (!$activeConnectionId) return;
    let columns: string[];
    let rows: unknown[][];
    let base: string;
    let sqlTable: string;
    if (whole) {
      if (tab.kind !== "table" || !tab.selected) {
        showToast(false, "Open a table to export all of its rows.");
        return;
      }
      const { schema, table } = tab.selected;
      base = table;
      sqlTable = qtable(schema, table);
      try {
        const r = await api.executeQuery($activeConnectionId, `SELECT * FROM ${sqlTable};`, tab.id);
        columns = r.columns;
        rows = r.rows;
      } catch (e) {
        showToast(false, (e as { message?: string })?.message ?? String(e));
        return;
      }
    } else {
      const r = tab.result;
      if (!r || r.columns.length === 0) {
        showToast(false, "Nothing to export — run a query or open a table first.");
        return;
      }
      columns = r.columns;
      rows = r.rows;
      base = tab.selected?.table ?? "result";
      sqlTable = tab.selected
        ? qtable(tab.selected.schema, tab.selected.table)
        : tab.editableTable
          ? qtable(tab.editableTable.schema, tab.editableTable.table)
          : qid("exported_table");
    }
    const path = await save({
      defaultPath: `${base}.csv`,
      filters: [
        { name: "CSV", extensions: ["csv"] },
        { name: "JSON", extensions: ["json"] },
        { name: "SQL", extensions: ["sql"] },
      ],
    });
    if (!path) return;
    const ext = path.split(".").pop()?.toLowerCase();
    const format = ext === "json" ? "json" : ext === "sql" ? "sql" : "csv";
    try {
      await api.writeTextFile(path, buildExport(format, columns, rows, sqlTable));
      showToast(true, `Exported ${rows.length} row${rows.length === 1 ? "" : "s"} to ${path}`);
    } catch (e) {
      showToast(false, (e as { message?: string })?.message ?? String(e));
    }
  }

  // ── Delete rows ─────────────────────────────────────────────────────────────
  async function deleteRows(e: CustomEvent<number[]>) {
    if ($readOnly) { showToast(false, blockedMsg); return; }
    const t = tab;
    const tbl = t.editableTable;
    if (!tbl || !$activeConnectionId || !t.result) return;
    const idxs = e.detail;
    if (!idxs.length) return;
    const ok = await ask(
      `Delete ${idxs.length} row${idxs.length === 1 ? "" : "s"} from ${tbl.schema}.${tbl.table}? This cannot be undone.`,
      { title: "Delete rows", kind: "warning" },
    );
    if (!ok) return;
    const cols = t.result.columns;
    const whereCols = t.pkColumns.length ? t.pkColumns.filter((c) => cols.includes(c)) : cols;
    t.running = true; t.error = null; sync();
    try {
      for (const i of idxs) {
        const row = t.result.rows[i];
        const where = whereCols
          .map((c) => {
            const v = row[cols.indexOf(c)];
            return v === null || v === undefined ? `${qid(c)} IS NULL` : `${qid(c)} = ${lit(v)}`;
          })
          .join(" AND ");
        const del = `DELETE FROM ${qtable(tbl.schema, tbl.table)} WHERE ${where};`;
        const s = performance.now();
        await api.executeQuery($activeConnectionId, del, t.id);
        logSql(del, { ms: Math.round(performance.now() - s) });
      }
    } catch (err) {
      t.error = String(err); t.running = false; sync();
      return;
    }
    t.running = false; t.selectedRow = null; sync();
    await browseTable(t, tbl.schema, tbl.table);
  }

  // ── Insert row ────────────────────────────────────────────────────────────
  function beginInsert() {
    if (tab.kind !== "table" || !tab.selected || tab.columns.length === 0) return;
    insertInitial = null;
    insertNonce += 1;
    inserting = true;
    detailOpen = true;
  }

  // Duplicate the selected row into a pre-filled insert form (PK columns cleared).
  function beginDuplicate() {
    if (tab.kind !== "table" || !tab.selected || tab.selectedRow === null || !tab.result) return;
    if (tab.columns.length === 0) return;
    const r = tab.result;
    const row = r.rows[tab.selectedRow];
    const init: Record<string, string | null> = {};
    for (const c of tab.columns) {
      if (tab.pkColumns.includes(c.name)) continue; // let serial/identity regenerate
      const i = r.columns.indexOf(c.name);
      if (i < 0) continue;
      const v = row[i];
      init[c.name] = v === null || v === undefined ? null : typeof v === "object" ? JSON.stringify(v) : String(v);
    }
    insertInitial = init;
    insertNonce += 1;
    inserting = true;
    detailOpen = true;
  }

  async function insertRow(e: CustomEvent<Record<string, string | null>>) {
    if ($readOnly) { showToast(false, blockedMsg); return; }
    const t = tab;
    const tbl = t.selected;
    if (!tbl || !$activeConnectionId) return;
    const updates = e.detail;
    const set = Object.keys(updates);
    const into = qtable(tbl.schema, tbl.table);
    const emptyInsert = $activeConnection?.kind === "mysql"
      ? `INSERT INTO ${into} () VALUES ();`
      : `INSERT INTO ${into} DEFAULT VALUES;`;
    const sql = set.length
      ? `INSERT INTO ${into} (${set.map(qid).join(", ")}) VALUES (${set.map((c) => lit(updates[c])).join(", ")});`
      : emptyInsert;
    t.running = true; t.error = null; sync();
    const start = performance.now();
    try {
      await api.executeQuery($activeConnectionId, sql, t.id);
      logSql(sql, { ms: Math.round(performance.now() - start) });
    } catch (err) {
      t.error = String(err); t.running = false; sync();
      logSql(sql, { ms: Math.round(performance.now() - start), error: String(err) });
      return;
    }
    t.running = false; inserting = false; sync();
    await browseTable(t, tbl.schema, tbl.table);
  }

  // ── Connection lifecycle (multi-workspace) ───────────────────────────────────
  // Each open connection keeps its own tab set; we stash the active one's tabs
  // before switching so they're restored when the user comes back.
  let addOpen = false;
  let stash: Record<string, { tabs: QueryTab[]; activeId: string; seq: number }> = {};

  function freshTabs() {
    seq = 1;
    tabs = [blankQueryTab()];
    activeId = tabs[0].id;
  }
  function stashCurrent() {
    if ($activeConnectionId) stash[$activeConnectionId] = { tabs, activeId, seq };
  }
  function restore(id: string) {
    const s = stash[id];
    if (s) {
      tabs = s.tabs; activeId = s.activeId; seq = s.seq;
    } else {
      freshTabs();
    }
  }
  async function reloadSidebar() {
    await tick();
    sidebar?.load();
  }

  function onConnected(e: CustomEvent<{ id: string; config: ConnectionConfig }>) {
    const { id, config } = e.detail;
    workspaces.update((w) => (w.some((x) => x.id === id) ? w : [...w, { id, config }]));
    stashCurrent();
    activeConnection.set(config);
    activeConnectionId.set(id);
    readOnly.set(shouldStartReadOnly(config.color, config.tag));
    freshTabs();
    schemaCatalog.set({});
    addOpen = false;
    reloadSidebar();
  }

  function switchWorkspace(id: string) {
    if (id === $activeConnectionId) return;
    const ws = $workspaces.find((w) => w.id === id);
    if (!ws) return;
    stashCurrent();
    activeConnection.set(ws.config);
    activeConnectionId.set(id);
    readOnly.set(shouldStartReadOnly(ws.config.color, ws.config.tag));
    restore(id);
    schemaCatalog.set({});
    reloadSidebar();
  }

  function closeWorkspace(id: string) {
    api.disconnect(id).catch(() => {});
    delete stash[id];
    const remaining = $workspaces.filter((w) => w.id !== id);
    workspaces.set(remaining);
    if ($activeConnectionId !== id) return; // closed a background workspace; nothing else to do
    if (remaining.length) {
      const next = remaining[0];
      activeConnection.set(next.config);
      activeConnectionId.set(next.id);
      readOnly.set(shouldStartReadOnly(next.config.color, next.config.tag));
      restore(next.id);
      schemaCatalog.set({});
      reloadSidebar();
    } else {
      activeConnectionId.set(null);
      activeConnection.set(null);
      schemaCatalog.set({});
      freshTabs();
    }
  }

  // Toolbar "close connection" acts on the active workspace.
  function disconnect() {
    if ($activeConnectionId) closeWorkspace($activeConnectionId);
  }

  async function refresh() {
    await sidebar?.load();
    if (tab.selected) await browseTable(tab, tab.selected.schema, tab.selected.table);
  }

  // ── Native menu (ids emitted from the Rust menu) ────────────────────────────
  function handleMenu(id: string) {
    switch (id) {
      case "new_query_tab": case "new_sql": newTab(); break;
      case "new_table": sidebar?.openAddTable(); break;
      case "close_tab": if (tabs.length > 1) closeTab(activeId); break;
      case "new_connection": addOpen = true; break;
      case "switch_schema": sidebar?.focusSchema(); break;
      case "run_query": if (tab.kind === "query") runSql(tab, tab.doc); break;
      case "cancel_query": cancelActive(); break;
      case "explain": explainQuery(); break;
      case "gen_select": generateSql("select"); break;
      case "gen_insert": generateSql("insert"); break;
      case "gen_update": generateSql("update"); break;
      case "gen_create": generateSql("create"); break;
      case "export_result": doExport(false); break;
      case "export_table": doExport(true); break;
      case "import_csv": openCsvImport(); break;
      case "export_db": openExport(); break;
      case "import_db": runImport(); break;
      case "refresh": refresh(); break;
      case "disconnect": disconnect(); break;
      case "open_palette": paletteOpen = true; break;
      case "force_reload": window.location.reload(); break;
      case "data_view": if (tab.kind === "table") setView("data"); break;
      case "structure_view": if (tab.kind === "table") setView("structure"); break;
      case "toggle_sidebar": sidebarOpen = !sidebarOpen; break;
      case "toggle_detail": detailOpen = !detailOpen; break;
      case "toggle_log": logOpen = !logOpen; break;
      case "commit": grid?.commitStaged(); break;
      case "add_row": beginInsert(); break;
      case "duplicate_row": beginDuplicate(); break;
      case "format_sql": if (tab.kind === "query") editor?.format(); break;
      case "prev_tab": cycleTab(-1); break;
      case "next_tab": cycleTab(1); break;
      case "settings": settingsOpen = true; break;
    }
  }

  let unlistenMenu: UnlistenFn | undefined;
  onMount(async () => {
    try {
      unlistenMenu = await listen<string>("menu", (e) => handleMenu(e.payload));
    } catch {
      /* not running under Tauri (browser dev) */
    }
  });
  onDestroy(() => unlistenMenu?.());

  // ── Keyboard shortcuts (the rest stay in the frontend; menu owns the globals) ─
  function isTextField(el: Element | null): boolean {
    if (!el) return false;
    const tag = el.tagName;
    return tag === "INPUT" || tag === "TEXTAREA" || (el as HTMLElement).isContentEditable;
  }
  function cycleTab(dir: number) {
    const i = tabs.findIndex((t) => t.id === activeId);
    activeId = tabs[(i + dir + tabs.length) % tabs.length].id;
  }

  function onKey(e: KeyboardEvent) {
    const meta = e.metaKey;
    const ctrl = e.ctrlKey;
    const shift = e.shiftKey;
    const field = isTextField(document.activeElement);

    // Esc closes overlays.
    if (e.key === "Escape") {
      if (paletteOpen) { paletteOpen = false; return; }
      if (settingsOpen) { settingsOpen = false; return; }
    }

    // Space → toggle the row-detail panel (when not typing).
    if (!meta && !ctrl && !shift && e.key === " " && !field && (tab.result?.rows.length ?? 0) > 0) {
      e.preventDefault();
      detailOpen = !detailOpen;
      if (detailOpen && tab.selectedRow === null) { tab.selectedRow = 0; sync(); }
      return;
    }

    if (!meta) return;

    // ⌘⌃[ / ⌘⌃] → Data / Structure view (no menu accelerator, so handled here).
    if (ctrl && (e.key === "[" || e.key === "]")) {
      if (tab.kind === "table") { e.preventDefault(); setView(e.key === "[" ? "data" : "structure"); }
      return;
    }
    // ⌘1..9 → jump to tab N.
    if (!ctrl && !shift && /^[1-9]$/.test(e.key)) {
      const i = parseInt(e.key, 10) - 1;
      if (tabs[i]) { e.preventDefault(); activeId = tabs[i].id; }
      return;
    }
    // ⌘F → toggle filters on a table tab (the editor keeps its own find elsewhere).
    if (!ctrl && !shift && e.key.toLowerCase() === "f" && tab.kind === "table") {
      e.preventDefault();
      tab.filtersOpen = !tab.filtersOpen;
      sync();
    }
    // Everything else (⌘T/E/W/P/R/K/S/I/N, ⌘[/], run, commit, …) is owned by the
    // native menu, which emits a "menu" event handled in handleMenu().
  }
</script>

<svelte:window on:keydown={onKey} />

{#if $workspaces.length === 0}
  <Welcome on:connected={onConnected} />
{:else}
  <div class="shell">
    <WorkspaceRail
      workspaces={$workspaces}
      activeId={$activeConnectionId}
      on:switch={(e) => switchWorkspace(e.detail)}
      on:add={() => (addOpen = true)}
      on:close={(e) => closeWorkspace(e.detail)}
    />
    <div class="app">
    <Toolbar
      {sidebarOpen}
      {logOpen}
      {detailOpen}
      readOnly={$readOnly}
      on:disconnect={disconnect}
      on:refresh={refresh}
      on:toggleSidebar={() => (sidebarOpen = !sidebarOpen)}
      on:toggleLog={() => (logOpen = !logOpen)}
      on:toggleDetail={() => (detailOpen = !detailOpen)}
      on:toggleReadOnly={() => readOnly.update((v) => !v)}
    />
    <div class="body" class:collapsed={!sidebarOpen}>
      {#if sidebarOpen}
        <Sidebar bind:this={sidebar} on:selectTable={onSelectTable} />
      {/if}
      <main class="main">
        <QueryTabs
          {tabs}
          {activeId}
          on:select={(e) => (activeId = e.detail)}
          on:close={(e) => closeTab(e.detail)}
          on:new={newTab}
        />

        {#if tab.kind === "query"}
          {#key activeId}
            <QueryEditor
              bind:this={editor}
              running={tab.running}
              dialect={$activeConnection?.kind ?? "postgres"}
              schema={$schemaCatalog}
              initialDoc={tab.doc}
              on:run={(e) => runSql(tab, e.detail)}
              on:change={(e) => (tab.doc = e.detail)}
            />
          {/key}
        {:else}
          <div class="subtabs">
            <button class="tab" class:active={tab.view === "data"} on:click={() => setView("data")}>Data</button>
            <button class="tab" class:active={tab.view === "structure"} on:click={() => setView("structure")}>Structure</button>
            {#if tab.selected}<span class="cur">{tab.selected.schema}.{tab.selected.table}</span>{/if}
            <div class="sub-spacer"></div>
            {#if tab.running}
              <button class="tab cancel-run" on:click={cancelActive} title="Cancel query (⌘.)">
                <span class="run-dot"></span> Cancel
              </button>
            {/if}
            {#if tab.view === "data"}
              {#if editing}
                <button class="tab addrow" on:click={beginInsert} title="Insert a new row">
                  <svg viewBox="0 0 14 14" width="12" height="12" aria-hidden="true"><path d="M7 3v8M3 7h8" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
                  Row
                </button>
              {/if}
              <button class="tab" class:active={tab.filtersOpen} on:click={() => { tab.filtersOpen = !tab.filtersOpen; sync(); }}>
                Filters{tab.filters.length ? ` (${tab.filters.length})` : ""}
              </button>
            {/if}
          </div>

          {#if tab.view === "data" && tab.filtersOpen}
            <FilterBar
              columns={tab.result?.columns?.length ? tab.result.columns : tab.columns.map((c) => c.name)}
              filters={tab.filters}
              on:apply={applyFilters}
              on:clear={clearFilters}
            />
          {/if}
        {/if}

        {#if tab.error}<div class="error-banner">{tab.error}</div>{/if}

        <div class="result">
          {#if tab.kind === "table" && tab.view === "structure"}
            <StructureView
              columns={tab.columns}
              schema={tab.selected?.schema ?? ""}
              table={tab.selected?.table ?? ""}
              kind={$activeConnection?.kind ?? "postgres"}
              connectionId={$activeConnectionId}
              on:changed={() => loadColumns(tab)}
            />
          {:else}
            <div class="data-area">
              <div class="main-col">
                <div class="grid-col">
                  {#if tab.results.length > 1}
                    <div class="rset-bar">
                      {#each tab.results as rs, ri (ri)}
                        <button class="rset" class:active={tab.resultIdx === ri} on:click={() => selectResult(tab, ri)}>
                          #{ri + 1}<span class="rset-n">{rs.columns.length ? `${rs.row_count} row${rs.row_count === 1 ? "" : "s"}` : "OK"}</span>
                        </button>
                      {/each}
                    </div>
                  {/if}
                  <DataGrid
                    bind:this={grid}
                    result={tab.result}
                    editable={editing}
                    altRows={$settings.altRows}
                    selectedRow={tab.selectedRow}
                    sort={tab.sort}
                    sortable={tab.kind === "table"}
                    tableKey={tab.selected ? `${tab.selected.schema}.${tab.selected.table}` : ""}
                    fkColumns={new Set(tab.foreignKeys.map((f) => f.column))}
                    on:followFk={followFk}
                    on:commit={commitEdits}
                    on:selectRow={(e) => { tab.selectedRow = e.detail; inserting = false; detailOpen = true; sync(); }}
                    on:sortColumn={(e) => toggleSort(e.detail)}
                    on:deleteRows={deleteRows}
                  />
                  {#if tab.kind === "table" && tab.result}
                    <div class="page-bar">
                      <span class="prange">Rows {tab.result.rows.length ? tab.offset + 1 : 0}–{tab.offset + tab.result.rows.length}</span>
                      <div class="sub-spacer"></div>
                      <button class="pbtn" disabled={tab.offset === 0 || tab.running} on:click={pagePrev}>‹ Prev</button>
                      <button class="pbtn" disabled={tab.result.rows.length < $settings.rowLimit || tab.running} on:click={pageNext}>Next ›</button>
                    </div>
                  {/if}
                </div>
                {#if logOpen}
                  <div class="log-col">
                    <LogPanel on:close={() => (logOpen = false)} on:pick={(e) => openHistoryQuery(e.detail)} />
                  </div>
                {/if}
              </div>
              {#if (detailOpen && tab.result) || inserting}
                <div class="detail-col">
                  <RowDetail
                    result={tab.result}
                    index={tab.selectedRow}
                    columns={tab.columns}
                    editable={editing}
                    insert={inserting}
                    initial={insertInitial}
                    {insertNonce}
                    on:commit={commitEdits}
                    on:insert={insertRow}
                    on:close={() => { detailOpen = false; inserting = false; }}
                  />
                </div>
              {/if}
            </div>
          {/if}
        </div>
      </main>
    </div>

    {#if paletteOpen}
      <CommandPalette
        on:select={(e) => { paletteOpen = false; openTable(e.detail.schema, e.detail.table); }}
        on:close={() => (paletteOpen = false)}
      />
    {/if}
    </div>
  </div>

  {#if addOpen}
    <div class="add-overlay">
      <Welcome dismissable on:connected={onConnected} on:cancel={() => (addOpen = false)} />
    </div>
  {/if}
{/if}

{#if settingsOpen}
  <Settings on:close={() => (settingsOpen = false)} />
{/if}

{#if exportOpen && $activeConnection}
  <ExportDialog cfg={$activeConnection} on:close={() => (exportOpen = false)} />
{/if}

{#if csvImportOpen && tab.selected}
  <ImportDialog
    columns={tab.columns}
    schema={tab.selected.schema}
    table={tab.selected.table}
    on:import={runCsvImport}
    on:close={() => (csvImportOpen = false)}
  />
{/if}

{#if toast}
  <div class="toast" class:err={!toast.ok} role="status">
    <span class="tmsg">{toast.msg}</span>
    <button class="tx" on:click={() => (toast = null)} aria-label="Dismiss">
      <svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true"><path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
    </button>
  </div>
{/if}

<style>
  .shell { display: flex; height: 100vh; overflow: hidden; }
  .app { flex: 1; min-width: 0; display: grid; grid-template-rows: 44px 1fr; overflow: hidden; }
  .add-overlay { position: fixed; inset: 0; z-index: var(--z-modal); background: var(--bg-content); }

  .toast {
    position: fixed; right: var(--s-5); bottom: var(--s-5); z-index: var(--z-toast);
    max-width: 460px; display: flex; align-items: flex-start; gap: var(--s-3);
    padding: var(--s-3) var(--s-4); border-radius: var(--r-md);
    background: var(--bg-elevated); border: 1px solid var(--border-strong);
    box-shadow: var(--shadow-pop); color: var(--ink-soft);
  }
  .toast.err { border-color: color-mix(in srgb, var(--danger) 40%, transparent); color: var(--danger); }
  .tmsg { font-size: 12px; white-space: pre-wrap; word-break: break-word; }
  .tx { flex: none; color: inherit; opacity: 0.7; }
  .tx:hover { opacity: 1; }
  .body { display: grid; grid-template-columns: 248px 1fr; grid-template-rows: minmax(0, 1fr); min-height: 0; overflow: hidden; }
  .body.collapsed { grid-template-columns: 1fr; }

  .main { display: flex; flex-direction: column; min-width: 0; min-height: 0; overflow: hidden; }

  .subtabs {
    display: flex; align-items: center; gap: var(--s-1);
    padding: var(--s-2) var(--s-4); background: var(--bg-panel);
    border-bottom: 1px solid var(--hairline); flex: none;
  }
  .tab {
    height: 24px; padding: 0 var(--s-4); border-radius: var(--r-sm);
    font-size: 12px; font-weight: 500; color: var(--muted);
    transition: background var(--t-fast) var(--ease-out), color var(--t-fast) var(--ease-out);
  }
  .tab:hover { color: var(--ink); }
  .tab.active { background: var(--bg-elevated); color: var(--ink); }
  .addrow { display: inline-flex; align-items: center; gap: 4px; color: var(--accent); }
  .addrow:hover { color: var(--accent); background: var(--bg-elevated); }
  .cur { margin-left: var(--s-3); font-size: 11.5px; color: var(--faint); font-family: var(--font-mono); }

  .result { flex: 1; display: flex; flex-direction: column; min-height: 0; }
  .sub-spacer { flex: 1; }
  .cancel-run { color: var(--danger) !important; display: inline-flex; align-items: center; gap: var(--s-2); }
  .cancel-run:hover { background: var(--danger-soft) !important; }
  .run-dot { width: 7px; height: 7px; border-radius: 50%; background: var(--danger); animation: runpulse 1s var(--ease-out) infinite; }
  @keyframes runpulse { 50% { opacity: 0.3; } }
  @media (prefers-reduced-motion: reduce) { .run-dot { animation: none; } }
  .data-area { flex: 1; display: flex; min-height: 0; min-width: 0; }
  .main-col { flex: 1; display: flex; flex-direction: column; min-width: 0; min-height: 0; }
  .grid-col { flex: 1; display: flex; flex-direction: column; min-width: 0; min-height: 0; }
  .rset-bar { display: flex; align-items: center; gap: var(--s-2); padding: var(--s-2) var(--s-4); background: var(--bg-panel); border-bottom: 1px solid var(--hairline); flex: none; overflow-x: auto; }
  .rset { display: inline-flex; align-items: center; gap: 5px; padding: 2px var(--s-3); border-radius: var(--r-sm); font-size: 11.5px; color: var(--muted); border: 1px solid transparent; white-space: nowrap; }
  .rset:hover { background: var(--bg-elevated); color: var(--ink); }
  .rset.active { color: var(--ink); background: var(--bg-elevated); border-color: var(--border); }
  .rset-n { font-size: 10px; color: var(--faint); }
  .page-bar { display: flex; align-items: center; gap: var(--s-3); padding: var(--s-2) var(--s-4); background: var(--bg-panel); border-top: 1px solid var(--hairline); flex: none; }
  .prange { font-size: 11.5px; color: var(--muted); font-family: var(--font-mono); }
  .pbtn { height: 24px; padding: 0 var(--s-3); border-radius: var(--r-sm); font-size: 12px; color: var(--ink-soft); border: 1px solid var(--border); background: var(--bg-content); }
  .pbtn:hover:not(:disabled) { background: var(--bg-elevated); color: var(--ink); }
  .pbtn:disabled { opacity: 0.4; }
  .log-col { height: 220px; flex: none; display: flex; min-height: 0; }
  .detail-col { width: 320px; flex: none; display: flex; min-height: 0; overflow: hidden; }

  .error-banner {
    margin: var(--s-3) var(--s-5); padding: var(--s-3) var(--s-4); border-radius: var(--r-sm);
    background: var(--danger-soft); color: var(--danger);
    border: 1px solid color-mix(in srgb, var(--danger) 30%, transparent);
    font-family: var(--font-mono); font-size: 11.5px; white-space: pre-wrap; flex: none;
  }
</style>
