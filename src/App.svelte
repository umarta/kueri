<script lang="ts">
  import { tick, onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
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
  import { settings } from "./lib/stores/settings";
  import {
    activeConnectionId, activeConnection, schemaCatalog, catalogColumns, workspaces, activeSchema,
  } from "./lib/stores/connection";
  import { api } from "./lib/tauri";
  import { logSql } from "./lib/stores/log";
  import type { ConnectionConfig, RowEdit, QueryTab } from "./lib/types";

  let sidebarOpen = true;
  let sidebar: Sidebar;
  let grid: DataGrid;
  let paletteOpen = false;
  let logOpen = false;
  let detailOpen = false;
  let inserting = false;
  let settingsOpen = false;

  // ── Query tabs ──────────────────────────────────────────────────────────────
  let seq = 1;
  function blankQueryTab(): QueryTab {
    return {
      id: crypto.randomUUID(), kind: "query", title: `Query ${seq++}`, doc: "SELECT now();",
      result: null, error: null, running: false, view: "data",
      selected: null, editableTable: null, pkColumns: [], columns: [],
      filters: [], filtersOpen: false, selectedRow: null,
    };
  }
  function tableTab(schema: string, table: string): QueryTab {
    return {
      id: crypto.randomUUID(), kind: "table", title: table, doc: "",
      result: null, error: null, running: false, view: "data",
      selected: { schema, table }, editableTable: null, pkColumns: [], columns: [],
      filters: [], filtersOpen: false, selectedRow: null,
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
      t.result = await api.executeQuery($activeConnectionId, sql);
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
  async function runSql(t: QueryTab, sql: string) {
    t.editableTable = null; t.pkColumns = []; t.columns = []; sync();
    await exec(t, sql);
    if (t.result) await resolveEditable(t);
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
  function buildWhere(t: QueryTab): string {
    if (!t.filters.length) return "";
    const parts = t.filters.map((f) => {
      const col = `"${f.column}"`;
      switch (f.op) {
        case "is null": return `${col} IS NULL`;
        case "is not null": return `${col} IS NOT NULL`;
        case "contains": return `${col}::text ILIKE ${lit("%" + f.value + "%")}`;
        case "starts": return `${col}::text ILIKE ${lit(f.value + "%")}`;
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
    t.doc = `SELECT * FROM "${schema}"."${table}"${buildWhere(t)} LIMIT ${$settings.rowLimit};`;
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
    } else {
      t = tableTab(schema, table);
      tabs = [...tabs, t];
    }
    activeId = t.id;
    t.selectedRow = null;
    sync();
    return browseTable(t, schema, table);
  }

  // ── Filters ───────────────────────────────────────────────────────────────
  function applyFilters(e: CustomEvent<import("./lib/types").FilterCond[]>) {
    if (!tab.selected) return;
    tab.filters = e.detail;
    sync();
    browseTable(tab, tab.selected.schema, tab.selected.table);
  }
  function clearFilters() {
    if (!tab.selected) return;
    tab.filters = [];
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
    const t = tab;
    const tbl = t.editableTable;
    if (!tbl || !$activeConnectionId || !t.result) return;
    const cols = t.result.columns;
    const whereCols = t.pkColumns.length ? t.pkColumns.filter((c) => cols.includes(c)) : cols;
    t.running = true; t.error = null; sync();
    try {
      for (const ch of e.detail) {
        const sets = Object.entries(ch.updates)
          .map(([col, val]) => `"${col}" = ${lit(val)}`)
          .join(", ");
        const where = whereCols
          .map((col) => {
            const v = ch.original[cols.indexOf(col)];
            return v === null || v === undefined ? `"${col}" IS NULL` : `"${col}" = ${lit(v)}`;
          })
          .join(" AND ");
        const upd = `UPDATE "${tbl.schema}"."${tbl.table}" SET ${sets} WHERE ${where};`;
        const us = performance.now();
        await api.executeQuery($activeConnectionId, upd);
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

  // ── Insert row ────────────────────────────────────────────────────────────
  function beginInsert() {
    if (tab.kind !== "table" || !tab.selected || tab.columns.length === 0) return;
    inserting = true;
    detailOpen = true;
  }

  async function insertRow(e: CustomEvent<Record<string, string | null>>) {
    const t = tab;
    const tbl = t.selected;
    if (!tbl || !$activeConnectionId) return;
    const updates = e.detail;
    const set = Object.keys(updates);
    const sql = set.length
      ? `INSERT INTO "${tbl.schema}"."${tbl.table}" (${set.map((c) => `"${c}"`).join(", ")}) VALUES (${set.map((c) => lit(updates[c])).join(", ")});`
      : `INSERT INTO "${tbl.schema}"."${tbl.table}" DEFAULT VALUES;`;
    t.running = true; t.error = null; sync();
    const start = performance.now();
    try {
      await api.executeQuery($activeConnectionId, sql);
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
      case "refresh": refresh(); break;
      case "disconnect": disconnect(); break;
      case "open_palette": paletteOpen = true; break;
      case "data_view": if (tab.kind === "table") setView("data"); break;
      case "structure_view": if (tab.kind === "table") setView("structure"); break;
      case "toggle_sidebar": sidebarOpen = !sidebarOpen; break;
      case "toggle_detail": detailOpen = !detailOpen; break;
      case "toggle_log": logOpen = !logOpen; break;
      case "commit": grid?.commitStaged(); break;
      case "add_row": beginInsert(); break;
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
      on:disconnect={disconnect}
      on:refresh={refresh}
      on:toggleSidebar={() => (sidebarOpen = !sidebarOpen)}
      on:toggleLog={() => (logOpen = !logOpen)}
      on:toggleDetail={() => (detailOpen = !detailOpen)}
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
              columns={tab.result?.columns ?? tab.columns.map((c) => c.name)}
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
                  <DataGrid
                    bind:this={grid}
                    result={tab.result}
                    editable={editing}
                    altRows={$settings.altRows}
                    selectedRow={tab.selectedRow}
                    on:commit={commitEdits}
                    on:selectRow={(e) => { tab.selectedRow = e.detail; inserting = false; detailOpen = true; sync(); }}
                  />
                </div>
                {#if logOpen}
                  <div class="log-col">
                    <LogPanel on:close={() => (logOpen = false)} />
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

<style>
  .shell { display: flex; height: 100vh; overflow: hidden; }
  .app { flex: 1; min-width: 0; display: grid; grid-template-rows: 44px 1fr; overflow: hidden; }
  .add-overlay { position: fixed; inset: 0; z-index: var(--z-modal); background: var(--bg-content); }
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
  .data-area { flex: 1; display: flex; min-height: 0; min-width: 0; }
  .main-col { flex: 1; display: flex; flex-direction: column; min-width: 0; min-height: 0; }
  .grid-col { flex: 1; display: flex; flex-direction: column; min-width: 0; min-height: 0; }
  .log-col { height: 220px; flex: none; display: flex; min-height: 0; }
  .detail-col { width: 320px; flex: none; display: flex; min-height: 0; overflow: hidden; }

  .error-banner {
    margin: var(--s-3) var(--s-5); padding: var(--s-3) var(--s-4); border-radius: var(--r-sm);
    background: var(--danger-soft); color: var(--danger);
    border: 1px solid color-mix(in srgb, var(--danger) 30%, transparent);
    font-family: var(--font-mono); font-size: 11.5px; white-space: pre-wrap; flex: none;
  }
</style>
