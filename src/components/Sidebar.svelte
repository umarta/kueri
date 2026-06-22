<script lang="ts">
  import { onMount, createEventDispatcher } from "svelte";
  import { api } from "../lib/tauri";
  import { activeConnectionId, activeConnection, catalogTables, activeSchema as activeSchemaStore } from "../lib/stores/connection";
  import { typeOptions, defaultIdColumn, type ColumnDraft } from "../lib/ddl";
  import Modal from "./Modal.svelte";
  import type { SchemaInfo, TableInfo, DbKind } from "../lib/types";

  const dispatch = createEventDispatcher<{ selectTable: { schema: string; table: string } }>();

  let schemas: SchemaInfo[] = [];
  let activeSchema = "";
  // Mirror the selected schema to a store so query tabs can resolve unqualified tables.
  $: activeSchemaStore.set(activeSchema);
  let tables: TableInfo[] = [];
  let loading = true;
  let filter = "";
  let selectedKey = "";
  let schemaSelectEl: HTMLSelectElement;

  /** ⌘K — focus the schema switcher (the closest analog to TablePlus "switch database"). */
  export function focusSchema() {
    schemaSelectEl?.focus();
  }

  /** File → New Table… (from the native menu). */
  export function openAddTable() {
    openAdd();
  }

  $: kind = ($activeConnection?.kind ?? "postgres") as DbKind;

  // Add-table (column designer)
  let showAdd = false;
  let newName = "";
  let cols: ColumnDraft[] = [];
  let creating = false;
  let createError = "";

  // Rename
  let showRename = false;
  let renameTarget = "";
  let renameValue = "";
  let renaming = false;
  let renameError = "";

  // Drop
  let showDrop = false;
  let dropTarget = "";
  let dropping = false;
  let dropError = "";

  // Duplicate
  let showDup = false;
  let dupTarget = "";
  let dupValue = "";
  let duplicating = false;
  let dupError = "";

  // Truncate
  let showTruncate = false;
  let truncTarget = "";
  let truncating = false;
  let truncError = "";

  onMount(load);

  export async function load() {
    if (!$activeConnectionId) return;
    loading = true;
    try {
      schemas = await api.listSchemas($activeConnectionId);
      activeSchema = schemas.find((s) => s.name === "public")?.name ?? schemas[0]?.name ?? "";
      await loadTables();
      // If the default schema is empty but another has tables, jump there
      // (e.g. tables live in "kame", not "public").
      if (tables.length === 0 && schemas.length > 1) {
        for (const s of schemas) {
          if (s.name === activeSchema) continue;
          const t = await api.listTables($activeConnectionId, s.name);
          if (t.length) {
            activeSchema = s.name;
            tables = t;
            catalogTables(t.map((x) => x.name));
            break;
          }
        }
      }
    } finally {
      loading = false;
    }
  }

  async function loadTables() {
    if (!activeSchema || !$activeConnectionId) {
      tables = [];
      return;
    }
    tables = await api.listTables($activeConnectionId, activeSchema);
    catalogTables(tables.map((t) => t.name));
  }

  async function onSchemaChange() {
    selectedKey = "";
    loading = true;
    try {
      await loadTables();
    } finally {
      loading = false;
    }
  }

  function select(schema: string, table: string) {
    selectedKey = `${schema}.${table}`;
    dispatch("selectTable", { schema, table });
  }

  $: visibleTables = filter
    ? tables.filter((t) => t.name.toLowerCase().includes(filter.toLowerCase()))
    : tables;

  // ── Add table ───────────────────────────────────────────────────────────────
  function openAdd() {
    newName = "";
    createError = "";
    cols = [defaultIdColumn(kind)];
    showAdd = true;
  }
  function addCol() {
    const opts = typeOptions(kind);
    cols = [...cols, { name: "", type: opts[2] ?? opts[0], nullable: true, primaryKey: false, default: "" }];
  }
  function removeCol(i: number) {
    cols = cols.filter((_, idx) => idx !== i);
  }
  async function createTable() {
    const name = newName.trim();
    if (!name || !activeSchema || !$activeConnectionId || creating) return;
    if (!cols.some((c) => c.name.trim())) {
      createError = "Add at least one column.";
      return;
    }
    creating = true;
    createError = "";
    try {
      await api.createTable($activeConnectionId, activeSchema, name, cols);
      showAdd = false;
      await loadTables();
      select(activeSchema, name);
    } catch (e) {
      createError = (e as { message?: string })?.message ?? String(e);
    } finally {
      creating = false;
    }
  }

  // ── Rename table ──────────────────────────────────────────────────────────────
  function openRename(table: string) {
    renameTarget = table;
    renameValue = table;
    renameError = "";
    showRename = true;
  }
  async function doRename() {
    const next = renameValue.trim();
    if (!next || next === renameTarget || !$activeConnectionId || renaming) {
      if (next === renameTarget) showRename = false;
      return;
    }
    renaming = true;
    renameError = "";
    try {
      await api.renameTable($activeConnectionId, activeSchema, renameTarget, next);
      showRename = false;
      if (selectedKey === `${activeSchema}.${renameTarget}`) select(activeSchema, next);
      await loadTables();
    } catch (e) {
      renameError = (e as { message?: string })?.message ?? String(e);
    } finally {
      renaming = false;
    }
  }

  // ── Drop table ────────────────────────────────────────────────────────────────
  function openDrop(table: string) {
    dropTarget = table;
    dropError = "";
    showDrop = true;
  }
  async function doDrop() {
    if (!$activeConnectionId || dropping) return;
    dropping = true;
    dropError = "";
    try {
      await api.dropTable($activeConnectionId, activeSchema, dropTarget);
      showDrop = false;
      await loadTables();
    } catch (e) {
      dropError = (e as { message?: string })?.message ?? String(e);
    } finally {
      dropping = false;
    }
  }

  // ── Duplicate table ───────────────────────────────────────────────────────────
  function openDup(table: string) {
    dupTarget = table;
    dupValue = `${table}_copy`;
    dupError = "";
    showDup = true;
  }
  async function doDuplicate() {
    const next = dupValue.trim();
    if (!next || !$activeConnectionId || duplicating) return;
    duplicating = true;
    dupError = "";
    try {
      await api.duplicateTable($activeConnectionId, activeSchema, dupTarget, next);
      showDup = false;
      await loadTables();
      select(activeSchema, next);
    } catch (e) {
      dupError = (e as { message?: string })?.message ?? String(e);
    } finally {
      duplicating = false;
    }
  }

  // ── Truncate table ────────────────────────────────────────────────────────────
  function openTruncate(table: string) {
    truncTarget = table;
    truncError = "";
    showTruncate = true;
  }
  async function doTruncate() {
    if (!$activeConnectionId || truncating) return;
    truncating = true;
    truncError = "";
    try {
      await api.truncateTable($activeConnectionId, activeSchema, truncTarget);
      showTruncate = false;
    } catch (e) {
      truncError = (e as { message?: string })?.message ?? String(e);
    } finally {
      truncating = false;
    }
  }
</script>

<aside class="sidebar">
  <div class="schemabar">
    <div class="schema-wrap">
      <svg class="dbicon" viewBox="0 0 16 16" width="13" height="13" aria-hidden="true">
        <ellipse cx="8" cy="4" rx="5" ry="2" fill="none" stroke="currentColor" stroke-width="1.2"/>
        <path d="M3 4v8c0 1.1 2.24 2 5 2s5-.9 5-2V4" fill="none" stroke="currentColor" stroke-width="1.2"/>
        <path d="M3 8c0 1.1 2.24 2 5 2s5-.9 5-2" fill="none" stroke="currentColor" stroke-width="1.2"/>
      </svg>
      <select
        class="schema-select"
        bind:this={schemaSelectEl}
        bind:value={activeSchema}
        on:change={onSchemaChange}
        disabled={!schemas.length}
        aria-label="Schema"
      >
        {#if !schemas.length}<option value="">—</option>{/if}
        {#each schemas as s (s.name)}<option value={s.name}>{s.name}</option>{/each}
      </select>
      <svg class="chev" viewBox="0 0 12 12" width="9" height="9" aria-hidden="true"><path d="M3 4.5L6 7.5l3-3" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>
    </div>
    <button class="add-btn" on:click={openAdd} disabled={!activeSchema} title="Add table" aria-label="Add table">
      <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M8 3.5v9M3.5 8h9" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>
    </button>
  </div>

  <div class="filter">
    <svg viewBox="0 0 16 16" width="13" height="13" aria-hidden="true"><circle cx="7" cy="7" r="4.5" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M11 11l3 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
    <input bind:value={filter} placeholder="Filter tables…" spellcheck="false" />
  </div>

  <nav class="tree" aria-label="Tables">
    {#if loading}
      {#each Array(6) as _, i (i)}<div class="skeleton" style="--w: {60 + ((i * 13) % 35)}%"></div>{/each}
    {:else}
      {#each visibleTables as t (t.name)}
        <div class="row" class:selected={selectedKey === `${activeSchema}.${t.name}`}>
          <button class="table" on:click={() => select(activeSchema, t.name)}>
            <svg class="ticon" viewBox="0 0 16 16" width="13" height="13" aria-hidden="true">
              <rect x="2.5" y="3" width="11" height="10" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.2"/>
              <line x1="2.5" y1="6.3" x2="13.5" y2="6.3" stroke="currentColor" stroke-width="1.2"/>
              <line x1="6.5" y1="6.3" x2="6.5" y2="13" stroke="currentColor" stroke-width="1.2"/>
            </svg>
            <span class="tname">{t.name}</span>
          </button>
          <div class="actions">
            <button class="act" title="Rename" aria-label="Rename {t.name}" on:click|stopPropagation={() => openRename(t.name)}>
              <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true"><path d="M10.5 2.5l3 3L6 13l-3.5.5L3 10z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/></svg>
            </button>
            <button class="act" title="Duplicate" aria-label="Duplicate {t.name}" on:click|stopPropagation={() => openDup(t.name)}>
              <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true"><rect x="5" y="5" width="8" height="8" rx="1.3" fill="none" stroke="currentColor" stroke-width="1.3"/><path d="M3 11V3.8c0-.4.4-.8.8-.8H11" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/></svg>
            </button>
            <button class="act" title="Truncate (delete all rows)" aria-label="Truncate {t.name}" on:click|stopPropagation={() => openTruncate(t.name)}>
              <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true"><path d="M2.5 8h11" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/><path d="M5 5.5l6 5M11 5.5l-6 5" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" opacity="0.55"/></svg>
            </button>
            <button class="act danger" title="Drop" aria-label="Drop {t.name}" on:click|stopPropagation={() => openDrop(t.name)}>
              <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true"><path d="M3 4.5h10M6.5 4V2.8h3V4M5 4.5l.5 8.5h5l.5-8.5" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </button>
          </div>
        </div>
      {/each}
      {#if visibleTables.length === 0}
        <p class="none">{filter ? "No matches" : "No tables"}</p>
      {/if}
    {/if}
  </nav>
</aside>

<!-- New table: column designer -->
{#if showAdd}
  <Modal title="New table" width="620px" on:close={() => (showAdd = false)}>
    <label class="sb-field">
      <span class="flabel">Name</span>
      <input class="finput" bind:value={newName} placeholder="e.g. customers" spellcheck="false" />
    </label>

    <div class="cols">
      <div class="cols-head">
        <span class="ch ch-name">Column</span>
        <span class="ch ch-type">Type</span>
        <span class="ch ch-def">Default</span>
        <span class="ch ch-flag">Null</span>
        <span class="ch ch-flag">PK</span>
        <span class="ch ch-x"></span>
      </div>
      {#each cols as c, i (i)}
        <div class="crow">
          <input class="ci ci-name" bind:value={c.name} placeholder="name" spellcheck="false" />
          <select class="ci ci-type" bind:value={c.type}>
            {#each typeOptions(kind) as opt (opt)}<option value={opt}>{opt}</option>{/each}
          </select>
          <input class="ci ci-def" bind:value={c.default} placeholder="—" spellcheck="false" />
          <label class="ci ci-flag"><input type="checkbox" bind:checked={c.nullable} /></label>
          <label class="ci ci-flag"><input type="checkbox" bind:checked={c.primaryKey} /></label>
          <button class="ci ci-x" title="Remove column" on:click={() => removeCol(i)} disabled={cols.length <= 1}>
            <svg viewBox="0 0 12 12" width="11" height="11" aria-hidden="true"><path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg>
          </button>
        </div>
      {/each}
      <button class="addcol" on:click={addCol}>
        <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true"><path d="M8 3.5v9M3.5 8h9" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
        Add column
      </button>
    </div>

    {#if createError}<p class="cerr">{createError}</p>{/if}
    <svelte:fragment slot="footer">
      <button class="btn ghost" on:click={() => (showAdd = false)}>Cancel</button>
      <button class="btn primary" on:click={createTable} disabled={!newName.trim() || creating}>
        {creating ? "Creating…" : "Create table"}
      </button>
    </svelte:fragment>
  </Modal>
{/if}

<!-- Rename -->
{#if showRename}
  <Modal title="Rename table" width="380px" on:close={() => (showRename = false)}>
    <label class="sb-field">
      <span class="flabel">New name</span>
      <input class="finput" bind:value={renameValue} spellcheck="false"
        on:keydown={(e) => { if (e.key === "Enter") doRename(); }} />
    </label>
    {#if renameError}<p class="cerr">{renameError}</p>{/if}
    <svelte:fragment slot="footer">
      <button class="btn ghost" on:click={() => (showRename = false)}>Cancel</button>
      <button class="btn primary" on:click={doRename} disabled={!renameValue.trim() || renaming}>
        {renaming ? "Renaming…" : "Rename"}
      </button>
    </svelte:fragment>
  </Modal>
{/if}

<!-- Drop confirm -->
{#if showDrop}
  <Modal title="Drop table" width="380px" on:close={() => (showDrop = false)}>
    <p class="fhint">
      Permanently drop <code>{activeSchema}.{dropTarget}</code> and all its data. This cannot be undone.
    </p>
    {#if dropError}<p class="cerr">{dropError}</p>{/if}
    <svelte:fragment slot="footer">
      <button class="btn ghost" on:click={() => (showDrop = false)}>Cancel</button>
      <button class="btn danger" on:click={doDrop} disabled={dropping}>
        {dropping ? "Dropping…" : "Drop table"}
      </button>
    </svelte:fragment>
  </Modal>
{/if}

<!-- Duplicate -->
{#if showDup}
  <Modal title="Duplicate table" width="380px" on:close={() => (showDup = false)}>
    <label class="sb-field">
      <span class="flabel">New table name</span>
      <input class="finput" bind:value={dupValue} spellcheck="false"
        on:keydown={(e) => { if (e.key === "Enter") doDuplicate(); }} />
    </label>
    <p class="fhint">Copies structure + data of <code>{dupTarget}</code> (indexes/constraints not copied).</p>
    {#if dupError}<p class="cerr">{dupError}</p>{/if}
    <svelte:fragment slot="footer">
      <button class="btn ghost" on:click={() => (showDup = false)}>Cancel</button>
      <button class="btn primary" on:click={doDuplicate} disabled={!dupValue.trim() || duplicating}>
        {duplicating ? "Duplicating…" : "Duplicate"}
      </button>
    </svelte:fragment>
  </Modal>
{/if}

<!-- Truncate confirm -->
{#if showTruncate}
  <Modal title="Truncate table" width="380px" on:close={() => (showTruncate = false)}>
    <p class="fhint">
      Delete <strong>all rows</strong> from <code>{activeSchema}.{truncTarget}</code> (structure kept). This cannot be undone.
    </p>
    {#if truncError}<p class="cerr">{truncError}</p>{/if}
    <svelte:fragment slot="footer">
      <button class="btn ghost" on:click={() => (showTruncate = false)}>Cancel</button>
      <button class="btn danger" on:click={doTruncate} disabled={truncating}>
        {truncating ? "Truncating…" : "Truncate"}
      </button>
    </svelte:fragment>
  </Modal>
{/if}

<style>
  .sidebar { display: flex; flex-direction: column; background: var(--bg-panel); border-right: 1px solid var(--border); min-width: 0; }

  .schemabar { display: flex; align-items: center; gap: var(--s-2); margin: var(--s-3) var(--s-3) var(--s-2); }
  .schema-wrap { position: relative; flex: 1; min-width: 0; display: flex; align-items: center; }
  .dbicon { position: absolute; left: var(--s-3); color: var(--faint); pointer-events: none; }
  .chev { position: absolute; right: var(--s-3); color: var(--faint); pointer-events: none; }
  .schema-select {
    flex: 1; min-width: 0; height: 28px; appearance: none; -webkit-appearance: none;
    padding: 0 22px 0 26px;
    background: var(--bg-content); border: 1px solid var(--border); border-radius: var(--r-sm);
    color: var(--ink); font: inherit; font-size: 12px; font-weight: 600;
    text-overflow: ellipsis; cursor: pointer;
    transition: border-color var(--t-fast) var(--ease-out);
  }
  .schema-select:focus { outline: none; border-color: var(--accent); }
  .schema-select:disabled { opacity: 0.6; cursor: default; }

  .add-btn {
    flex: none; width: 28px; height: 28px; display: grid; place-items: center;
    background: var(--bg-content); border: 1px solid var(--border); border-radius: var(--r-sm);
    color: var(--muted);
    transition: background var(--t-fast) var(--ease-out), color var(--t-fast) var(--ease-out), border-color var(--t-fast) var(--ease-out);
  }
  .add-btn:hover:not(:disabled) { background: var(--accent); color: var(--accent-ink); border-color: var(--accent); }
  .add-btn:disabled { opacity: 0.4; }

  .filter {
    display: flex; align-items: center; gap: var(--s-3);
    margin: 0 var(--s-3) var(--s-3); padding: 0 var(--s-3); height: 28px;
    background: var(--bg-content); border: 1px solid var(--border); border-radius: var(--r-sm);
    color: var(--faint);
    transition: border-color var(--t-fast) var(--ease-out);
  }
  .filter:focus-within { border-color: var(--accent); }
  .filter input { flex: 1; min-width: 0; background: none; border: none; outline: none; color: var(--ink); font: inherit; font-size: 12px; }
  .filter input::placeholder { color: var(--faint); }

  .tree { flex: 1; overflow-y: auto; padding: 0 var(--s-2) var(--s-3); }

  .row { display: flex; align-items: center; border-radius: var(--r-sm); }
  .row:hover { background: var(--bg-elevated); }
  .row.selected { background: var(--accent); }
  .table {
    flex: 1; min-width: 0; display: flex; align-items: center; gap: var(--s-3);
    padding: var(--s-2) var(--s-3);
    text-align: left; color: var(--ink-soft); font-size: 12.5px; background: none;
  }
  .row:hover .table { color: var(--ink); }
  .row.selected .table { color: var(--accent-ink); }
  .row.selected .ticon { color: var(--accent-ink); opacity: 0.9; }
  .ticon { color: var(--muted); flex: none; }
  .tname { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .actions { display: none; align-items: center; gap: 2px; padding-right: var(--s-2); flex: none; }
  .row:hover .actions, .row.selected .actions { display: flex; }
  .act { width: 20px; height: 20px; display: grid; place-items: center; border-radius: var(--r-xs); color: var(--muted); }
  .row.selected .act { color: var(--accent-ink); opacity: 0.85; }
  .act:hover { background: var(--bg-hover); color: var(--ink); }
  .act.danger:hover { background: color-mix(in srgb, var(--danger, #e5484d) 18%, transparent); color: var(--danger, #e5484d); }

  .none { margin: 0; padding: var(--s-2) var(--s-3); font-size: 11.5px; color: var(--faint); }

  .skeleton {
    height: 14px; margin: var(--s-3) var(--s-3); border-radius: var(--r-xs); width: var(--w);
    background: linear-gradient(90deg, var(--bg-elevated), var(--bg-hover), var(--bg-elevated));
    background-size: 200% 100%; animation: shimmer 1.3s var(--ease-out) infinite;
  }
  @keyframes shimmer { to { background-position: -200% 0; } }
  @media (prefers-reduced-motion: reduce) { .skeleton { animation: none; } }

  /* Modal fields */
  .sb-field { display: flex; flex-direction: column; gap: var(--s-2); }
  .flabel { font-size: 11px; font-weight: 600; color: var(--muted); text-transform: uppercase; letter-spacing: 0.04em; }
  .finput {
    height: 32px; padding: 0 var(--s-3);
    background: var(--bg-content); border: 1px solid var(--border); border-radius: var(--r-sm);
    color: var(--ink); font: inherit; font-size: 13px;
  }
  .finput:focus { outline: none; border-color: var(--accent); }
  .fhint { margin: 0; font-size: 12.5px; color: var(--ink-soft); line-height: 1.55; }
  .fhint code { font-family: var(--font-mono, ui-monospace, monospace); color: var(--ink); background: var(--bg-elevated); padding: 0 4px; border-radius: var(--r-xs); }
  .cerr { margin: var(--s-3) 0 0; font-size: 11.5px; color: var(--danger, #e5484d); line-height: 1.5; }

  /* Column designer */
  .cols { margin-top: var(--s-5); }
  .cols-head, .crow { display: grid; grid-template-columns: 1fr 1.1fr 0.9fr 44px 38px 26px; gap: var(--s-2); align-items: center; }
  .cols-head { padding: 0 0 var(--s-2); }
  .ch { font-size: 10.5px; font-weight: 600; color: var(--faint); text-transform: uppercase; letter-spacing: 0.03em; }
  .ch-flag { text-align: center; }
  .crow { margin-bottom: var(--s-2); }
  .ci-name, .ci-type, .ci-def {
    height: 30px; padding: 0 var(--s-3);
    background: var(--bg-content); border: 1px solid var(--border); border-radius: var(--r-sm);
    color: var(--ink); font: inherit; font-size: 12.5px;
  }
  .ci-def { font-family: var(--font-mono, ui-monospace, monospace); }
  .ci-type { appearance: none; -webkit-appearance: none; cursor: pointer; }
  .ci-name:focus, .ci-type:focus, .ci-def:focus { outline: none; border-color: var(--accent); }
  .ci-flag { display: grid; place-items: center; height: 30px; }
  .ci-flag input { width: 15px; height: 15px; accent-color: var(--accent); }
  .ci-x { display: grid; place-items: center; height: 30px; color: var(--faint); border-radius: var(--r-xs); }
  .ci-x:hover:not(:disabled) { color: var(--danger, #e5484d); }
  .ci-x:disabled { opacity: 0.3; }
  .addcol {
    display: inline-flex; align-items: center; gap: var(--s-2); margin-top: var(--s-2);
    padding: var(--s-2) var(--s-3); border-radius: var(--r-sm);
    font-size: 12px; font-weight: 600; color: var(--accent);
  }
  .addcol:hover { background: var(--bg-elevated); }

  .btn { height: 30px; padding: 0 var(--s-5); border-radius: var(--r-sm); font: inherit; font-size: 12.5px; font-weight: 600; border: 1px solid transparent; }
  .btn.ghost { background: transparent; border-color: var(--border); color: var(--ink-soft); }
  .btn.ghost:hover { background: var(--bg-elevated); }
  .btn.primary { background: var(--accent); color: var(--accent-ink); }
  .btn.primary:hover:not(:disabled) { filter: brightness(1.05); }
  .btn.primary:disabled { opacity: 0.5; }
  .btn.danger { background: var(--danger, #e5484d); color: #fff; }
  .btn.danger:hover:not(:disabled) { filter: brightness(1.05); }
  .btn.danger:disabled { opacity: 0.5; }
</style>
