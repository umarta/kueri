<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { api } from "../lib/tauri";
  import { typeOptions, alterTypeOptions, supportsColumnAlter, type ColumnDraft } from "../lib/ddl";
  import { readOnly } from "../lib/stores/connection";
  import type { ColumnInfo, DbKind, IndexInfo, ForeignKey } from "../lib/types";

  export let columns: ColumnInfo[] = [];
  export let schema = "";
  export let table = "";
  export let kind: DbKind = "postgres";
  export let connectionId: string | null = null;

  const dispatch = createEventDispatcher<{ changed: void }>();

  let busy = false;
  let error = "";

  // Add-column inline form
  let adding = false;
  let draft: ColumnDraft = { name: "", type: "", nullable: true, primaryKey: false, default: "" };

  // Inline rename
  let renaming: string | null = null;
  let renameVal = "";

  // Drop confirm
  let confirmDrop: string | null = null;

  // DDL (CREATE statement) viewer
  let ddlOpen = false;
  let ddlText = "";
  let ddlBusy = false;
  let ddlKey = "";
  $: if (`${schema}.${table}` !== ddlKey) { ddlKey = `${schema}.${table}`; ddlOpen = false; ddlText = ""; }
  async function toggleDdl() {
    ddlOpen = !ddlOpen;
    if (ddlOpen && connectionId && table) {
      ddlBusy = true;
      ddlText = "";
      try {
        ddlText = await api.tableDdl(connectionId, schema, table);
      } catch (e) {
        ddlText = "-- " + ((e as { message?: string })?.message ?? String(e));
      } finally {
        ddlBusy = false;
      }
    }
  }
  async function copyDdl() {
    try {
      await navigator.clipboard.writeText(ddlText);
    } catch {
      /* clipboard unavailable */
    }
  }

  // Indexes + foreign keys + primary keys
  let indexes: IndexInfo[] = [];
  let fks: ForeignKey[] = [];
  let pkColumns: string[] = [];
  let metaKey = "";
  $: if (connectionId && table && `${schema}.${table}` !== metaKey) {
    metaKey = `${schema}.${table}`;
    loadMeta();
  }
  async function loadMeta() {
    if (!connectionId || !table) {
      indexes = [];
      fks = [];
      pkColumns = [];
      return;
    }
    indexes = await api.listIndexes(connectionId, schema, table).catch(() => []);
    fks = await api.foreignKeys(connectionId, schema, table).catch(() => []);
    pkColumns = await api.primaryKeys(connectionId, schema, table).catch(() => []);
  }
  $: fkMap = new Map(fks.map((f) => [f.column, f]));
  $: pkSet = new Set(pkColumns);

  // Column search filter
  let colSearch = "";
  $: shownCols = colSearch.trim()
    ? columns.filter((c) => c.name.toLowerCase().includes(colSearch.trim().toLowerCase()))
    : columns;

  let addingIdx = false;
  let idxName = "";
  let idxCols: string[] = [];
  let idxUnique = false;
  function startAddIdx() {
    addingIdx = true;
    idxName = "";
    idxCols = [];
    idxUnique = false;
  }
  async function saveIdx() {
    if (!connectionId || !idxName.trim() || !idxCols.length) return;
    if (await run(() => api.createIndex(connectionId!, schema, table, idxName.trim(), idxCols, idxUnique))) {
      addingIdx = false;
      await loadMeta();
      dispatch("changed");
    }
  }
  async function dropIdx(name: string) {
    if (await run(() => api.dropIndex(connectionId!, schema, table, name))) await loadMeta();
  }

  // Foreign-key cell menu (view existing / create new)
  $: fkCreatable = canEdit && kind !== "sqlite";
  let fkMenu: { col: string; left: number; top: number } | null = null;
  let fkCreating = false;
  let fkTables: string[] = [];
  let fkRefCols: string[] = [];
  let fkRefTable = "";
  let fkRefCol = "";
  let fkValidate = true;
  function openFkMenu(c: ColumnInfo, e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    fkMenu = { col: c.name, left: Math.min(r.left, window.innerWidth - 280), top: r.bottom + 2 };
    fkCreating = false;
  }
  async function startCreateFk(col: string) {
    if (!connectionId) return;
    fkTables = (await api.listTables(connectionId, schema).catch(() => []))
      .map((t) => t.name)
      .filter((t) => t !== table);
    // Guess the referenced table from a `<thing>_id` column name.
    const base = col.endsWith("_id") ? col.slice(0, -3) : col;
    fkRefTable = fkTables.find((t) => t === base || t === base + "s") ?? fkTables[0] ?? "";
    fkValidate = true;
    fkCreating = true;
    await loadRefCols();
  }
  async function loadRefCols() {
    if (!connectionId || !fkRefTable) {
      fkRefCols = [];
      return;
    }
    fkRefCols = (await api.listColumns(connectionId, schema, fkRefTable).catch(() => [])).map((c) => c.name);
    fkRefCol = fkRefCols.includes("id") ? "id" : (fkRefCols[0] ?? "");
  }
  async function saveFk() {
    if (!fkMenu || !connectionId || !fkRefTable || !fkRefCol) return;
    const col = fkMenu.col;
    const name = `fk_${table}_${col}`;
    if (await run(() => api.addForeignKey(connectionId!, schema, table, col, fkRefTable, fkRefCol, name, fkValidate))) {
      fkMenu = null;
      fkCreating = false;
      await loadMeta();
    }
  }

  $: canEdit = !!(connectionId && schema && table) && !$readOnly;
  $: alterOk = canEdit && supportsColumnAlter(kind);

  // Inline type edit
  let typingCol: string | null = null;
  let typeVal = "";
  function startType(c: ColumnInfo) {
    if (!alterOk) return;
    error = "";
    typingCol = c.name;
    const opts = alterTypeOptions(kind);
    typeVal = opts.includes(c.data_type) ? c.data_type : opts[0];
  }
  async function commitType(c: ColumnInfo) {
    const col = typingCol;
    if (!col) return;
    if (!typeVal || typeVal === c.data_type) { typingCol = null; return; }
    await run(() => api.changeColumnType(connectionId!, schema, table, col, typeVal, !c.nullable));
    typingCol = null;
  }
  async function toggleNull(c: ColumnInfo) {
    if (!alterOk || busy) return;
    // c.nullable === true → currently NULL → make it NOT NULL (notNull = c.nullable).
    await run(() => api.setColumnNullable(connectionId!, schema, table, c.name, c.data_type, c.nullable));
  }

  function startAdd() {
    error = "";
    const opts = typeOptions(kind);
    draft = { name: "", type: opts[2] ?? opts[0], nullable: true, primaryKey: false, default: "" };
    adding = true;
  }

  async function run(op: () => Promise<unknown>): Promise<boolean> {
    if (!connectionId) return false;
    busy = true;
    error = "";
    try {
      await op();
      dispatch("changed");
      return true;
    } catch (e) {
      error = (e as { message?: string })?.message ?? String(e);
      return false;
    } finally {
      busy = false;
    }
  }

  async function addColumn() {
    if (!draft.name.trim() || !draft.type.trim()) return;
    if (await run(() => api.addColumn(connectionId!, schema, table, draft))) adding = false;
  }

  function startRename(name: string) {
    error = "";
    renaming = name;
    renameVal = name;
  }
  async function commitRename() {
    const old = renaming;
    const next = renameVal.trim();
    if (!old) return;
    if (!next || next === old) { renaming = null; return; }
    if (await run(() => api.renameColumn(connectionId!, schema, table, old, next))) renaming = null;
  }

  async function dropColumn() {
    const col = confirmDrop;
    if (!col) return;
    if (await run(() => api.dropColumn(connectionId!, schema, table, col))) confirmDrop = null;
  }
</script>

{#if columns.length || canEdit}
  <div class="sv">
    <!-- Header: table name + primary key chips + column search -->
    <div class="sv-head">
      <span class="hk">Name</span>
      <span class="tname">{table || "—"}</span>
      {#if pkColumns.length}
        <span class="hk">Primary</span>
        {#each pkColumns as p (p)}<span class="pk-chip">{p}</span>{/each}
      {/if}
      <div class="hspace"></div>
      {#if table}<button class="hbtn" class:on={ddlOpen} on:click={toggleDdl}>DDL</button>{/if}
      {#if canEdit}<button class="hbtn accent" on:click={startAdd} disabled={busy}>+ Column</button>{/if}
      <div class="search">
        <svg viewBox="0 0 16 16" width="13" height="13" aria-hidden="true"><circle cx="7" cy="7" r="4.5" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M11 11l3 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
        <input bind:value={colSearch} placeholder="Search for column…" spellcheck="false" />
      </div>
    </div>

    {#if error}<div class="err">{error}</div>{/if}

    {#if ddlOpen}
      <div class="ddl-panel">
        <div class="ddl-head">
          <span>CREATE statement</span>
          <button class="ddl-copy" on:click={copyDdl} disabled={!ddlText}>Copy</button>
        </div>
        {#if ddlBusy}<div class="ddl-loading">Loading…</div>{:else}<pre class="ddl-code">{ddlText}</pre>{/if}
      </div>
    {/if}

    <!-- Columns pane -->
    <div class="pane cols-pane">
      <table class="grid">
        <thead>
          <tr>
            <th class="gut">#</th><th>column_name</th><th>data_type</th><th>is_nullable</th>
            <th>check</th><th>column_default</th><th>foreign_key</th><th>comment</th><th class="ax"></th>
          </tr>
        </thead>
        <tbody>
          {#each shownCols as c (c.name)}
            {@const i = columns.indexOf(c)}
            <tr>
              <td class="gut">{i + 1}</td>
              <td class="cn">
                {#if pkSet.has(c.name)}<span class="keyi" title="Primary key">★</span>{/if}
                {#if renaming === c.name}
                  <!-- svelte-ignore a11y-autofocus -->
                  <input class="cin" bind:value={renameVal} autofocus spellcheck="false"
                    on:keydown={(e) => { if (e.key === "Enter") commitRename(); else if (e.key === "Escape") renaming = null; }}
                    on:blur={commitRename} />
                {:else}
                  <!-- svelte-ignore a11y-no-static-element-interactions -->
                  <span class="cn-text" class:editable={canEdit} on:dblclick={() => canEdit && startRename(c.name)}>{c.name}</span>
                {/if}
              </td>
              <td class="ty">
                {#if typingCol === c.name}
                  <!-- svelte-ignore a11y-autofocus -->
                  <select class="cin" bind:value={typeVal} autofocus on:change={() => commitType(c)} on:blur={() => (typingCol = null)}>
                    {#each alterTypeOptions(kind) as opt (opt)}<option value={opt}>{opt}</option>{/each}
                    {#if !alterTypeOptions(kind).includes(c.data_type)}<option value={c.data_type}>{c.data_type}</option>{/if}
                  </select>
                {:else if alterOk}
                  <button class="cellbtn" on:click={() => startType(c)} disabled={busy} title="Change type">{c.data_type}</button>
                {:else}{c.data_type}{/if}
              </td>
              <td>
                {#if alterOk}
                  <button class="pillbtn" on:click={() => toggleNull(c)} disabled={busy} title="Toggle nullability">
                    <span class="pill" class:yes={c.nullable}>{c.nullable ? "YES" : "NO"}</span>
                  </button>
                {:else}
                  <span class="pill" class:yes={c.nullable}>{c.nullable ? "YES" : "NO"}</span>
                {/if}
              </td>
              <td class="dim">—</td>
              <td class:dim={!c.default}>{c.default ?? "—"}</td>
              <td class="fk">
                {#if fkCreatable || fkMap.has(c.name)}
                  <button class="fk-cell" class:linked={fkMap.has(c.name)} on:click={(e) => openFkMenu(c, e)}>
                    {#if fkMap.has(c.name)}{fkMap.get(c.name)?.ref_table}({fkMap.get(c.name)?.ref_column}){:else}<span class="dim">EMPTY</span>{/if}
                  </button>
                {:else}
                  <span class="dim">{fkMap.has(c.name) ? "" : "—"}</span>
                {/if}
              </td>
              <td class="dim">—</td>
              <td class="ax">
                {#if canEdit}
                  <button class="act" title="Rename column" on:click={() => startRename(c.name)} disabled={busy}>
                    <svg viewBox="0 0 16 16" width="11" height="11" aria-hidden="true"><path d="M10.5 2.5l3 3L6 13l-3.5.5L3 10z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/></svg>
                  </button>
                  <button class="act danger" title="Drop column" on:click={() => (confirmDrop = c.name)} disabled={busy}>
                    <svg viewBox="0 0 16 16" width="11" height="11" aria-hidden="true"><path d="M3 4.5h10M6.5 4V2.8h3V4M5 4.5l.5 8.5h5l.5-8.5" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>
                  </button>
                {/if}
              </td>
            </tr>
          {/each}

          {#if adding}
            <tr class="addrow">
              <td class="gut">+</td>
              <td><input class="cin" bind:value={draft.name} placeholder="column_name" spellcheck="false"
                on:keydown={(e) => { if (e.key === "Enter") addColumn(); else if (e.key === "Escape") adding = false; }} /></td>
              <td>
                <select class="cin" bind:value={draft.type}>
                  {#each typeOptions(kind) as opt (opt)}<option value={opt}>{opt}</option>{/each}
                </select>
              </td>
              <td><label class="nullbox"><input type="checkbox" bind:checked={draft.nullable} /> NULL</label></td>
              <td class="addactions" colspan="5">
                <button class="btn primary sm" on:click={addColumn} disabled={busy || !draft.name.trim()}>Add</button>
                <button class="btn ghost sm" on:click={() => (adding = false)} disabled={busy}>Cancel</button>
              </td>
            </tr>
          {/if}

          {#if !shownCols.length && colSearch}
            <tr><td class="dim mid" colspan="9">No columns match “{colSearch}”.</td></tr>
          {/if}
        </tbody>
      </table>
    </div>

    <!-- Indexes pane -->
    {#if table}
      <div class="pane idx-pane">
        <div class="pane-bar">
          <span class="pane-title">Indexes</span>
          <span class="pane-count">{indexes.length}</span>
          <div class="hspace"></div>
          {#if canEdit && !addingIdx}<button class="hbtn accent" on:click={startAddIdx}>+ Index</button>{/if}
        </div>

        {#if addingIdx}
          <div class="idx-form">
            <input class="idx-input" bind:value={idxName} placeholder="index name" spellcheck="false" />
            <div class="idx-pick">
              {#each columns as c (c.name)}
                <label class="idx-chk"><input type="checkbox" bind:group={idxCols} value={c.name} />{c.name}</label>
              {/each}
            </div>
            <label class="idx-chk"><input type="checkbox" bind:checked={idxUnique} /> Unique</label>
            <div class="idx-actions">
              <button class="idx-cancel" on:click={() => (addingIdx = false)} disabled={busy}>Cancel</button>
              <button class="idx-save" on:click={saveIdx} disabled={busy || !idxName.trim() || !idxCols.length}>Create</button>
            </div>
          </div>
        {/if}

        <table class="grid">
          <thead>
            <tr>
              <th>index_name</th><th>index_algorithm</th><th>is_unique</th><th>column_name</th>
              <th>condition</th><th>include</th><th>comment</th><th class="ax"></th>
            </tr>
          </thead>
          <tbody>
            {#each indexes as ix (ix.name)}
              <tr>
                <td class="cn">{ix.name}</td>
                <td class="algo">{ix.method ? ix.method.toUpperCase() : "—"}</td>
                <td><span class="pill" class:yes={!ix.unique}>{ix.unique ? "TRUE" : "FALSE"}</span></td>
                <td class="mono">{ix.columns.join(", ")}</td>
                <td class:dim={!ix.predicate}>{ix.predicate || "—"}</td>
                <td class="dim">—</td>
                <td class="dim">—</td>
                <td class="ax">
                  {#if canEdit}
                    <button class="act danger" title="Drop index" on:click={() => dropIdx(ix.name)} disabled={busy}>
                      <svg viewBox="0 0 16 16" width="11" height="11" aria-hidden="true"><path d="M3 4.5h10M6.5 4V2.8h3V4M5 4.5l.5 8.5h5l.5-8.5" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>
                    </button>
                  {/if}
                </td>
              </tr>
            {/each}
            {#if !indexes.length}<tr><td class="dim mid" colspan="8">No indexes</td></tr>{/if}
          </tbody>
        </table>
      </div>
    {/if}
  </div>
{:else}
  <div class="empty">Select a table to inspect its structure.</div>
{/if}

{#if fkMenu}
  {@const m = fkMenu}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="fk-backdrop" on:click={() => (fkMenu = null)}></div>
  <div class="fk-menu" style="left: {m.left}px; top: {m.top}px;">
    {#if fkMap.has(m.col)}
      <div class="fk-existing">→ {fkMap.get(m.col)?.ref_schema}.{fkMap.get(m.col)?.ref_table}({fkMap.get(m.col)?.ref_column})</div>
    {/if}
    {#if fkCreatable}
      {#if !fkCreating}
        <button class="fk-create" on:click={() => startCreateFk(m.col)}>
          <span class="fk-plus">＋</span> Create a foreign key on <code>{m.col}</code>
        </button>
      {:else}
        <div class="fk-form">
          <label class="fk-l">References
            <select bind:value={fkRefTable} on:change={loadRefCols}>
              {#each fkTables as t (t)}<option value={t}>{t}</option>{/each}
            </select>
          </label>
          <label class="fk-l">Column
            <select bind:value={fkRefCol}>
              {#each fkRefCols as rc (rc)}<option value={rc}>{rc}</option>{/each}
            </select>
          </label>
          {#if kind === "postgres"}
            <label class="fk-skip"><input type="checkbox" bind:checked={fkValidate} /> Validate existing rows</label>
          {/if}
          {#if error}<p class="fk-err">{error}</p>{/if}
          {#if error && fkValidate && kind === "postgres" && /foreign key/i.test(error)}
            <p class="fk-tip">Existing rows violate this key — uncheck “Validate existing rows” to add it anyway (NOT VALID).</p>
          {/if}
          <div class="fk-act">
            <button class="idx-cancel" on:click={() => (fkMenu = null)} disabled={busy}>Cancel</button>
            <button class="idx-save" on:click={saveFk} disabled={busy || !fkRefTable || !fkRefCol}>Create</button>
          </div>
        </div>
      {/if}
    {/if}
  </div>
{/if}

{#if confirmDrop}
  <div class="cdrop-backdrop" role="button" tabindex="-1" on:click|self={() => (confirmDrop = null)} on:keydown={() => {}}>
    <div class="cdrop" role="dialog" aria-modal="true">
      <p>Drop column <code>{confirmDrop}</code> from <code>{table}</code>? This deletes its data.</p>
      {#if error}<p class="err inline">{error}</p>{/if}
      <div class="cdrop-foot">
        <button class="btn ghost" on:click={() => (confirmDrop = null)}>Cancel</button>
        <button class="btn danger" on:click={dropColumn} disabled={busy}>{busy ? "Dropping…" : "Drop column"}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .sv { height: 100%; display: flex; flex-direction: column; min-height: 0; background: var(--bg-content); }

  /* Header: name + primary key + column search */
  .sv-head {
    display: flex; align-items: center; gap: var(--s-3); flex: none;
    padding: var(--s-2) var(--s-4); background: var(--bg-panel); border-bottom: 1px solid var(--border);
  }
  .hk { font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--faint); }
  .tname { font-family: var(--font-mono); font-size: 12.5px; font-weight: 700; color: var(--ink); }
  .pk-chip { font-family: var(--font-mono); font-size: 11px; color: var(--accent); background: color-mix(in srgb, var(--accent) 16%, transparent); padding: 1px 7px; border-radius: var(--r-pill); }
  .hspace { flex: 1; }
  .hbtn { padding: var(--s-1) var(--s-3); border-radius: var(--r-sm); font-size: 11.5px; font-weight: 600; color: var(--muted); }
  .hbtn:hover:not(:disabled) { background: var(--bg-elevated); color: var(--ink); }
  .hbtn.on { color: var(--accent); background: var(--bg-elevated); }
  .hbtn.accent { color: var(--accent); }
  .hbtn:disabled { opacity: 0.5; }
  .search { display: flex; align-items: center; gap: var(--s-2); width: 200px; height: 26px; padding: 0 var(--s-2); background: var(--bg-content); border: 1px solid var(--border); border-radius: var(--r-sm); color: var(--faint); }
  .search input { flex: 1; min-width: 0; background: none; border: none; outline: none; color: var(--ink); font: inherit; font-size: 12px; }

  /* DDL panel */
  .ddl-panel { flex: none; border-bottom: 1px solid var(--hairline); background: var(--bg-content); }
  .ddl-head { display: flex; align-items: center; justify-content: space-between; padding: var(--s-2) var(--s-4); font-size: 11px; color: var(--faint); text-transform: uppercase; letter-spacing: 0.04em; }
  .ddl-copy { font-size: 11px; color: var(--accent); padding: 2px var(--s-2); border-radius: var(--r-xs); text-transform: none; letter-spacing: 0; }
  .ddl-copy:hover:not(:disabled) { background: var(--bg-elevated); }
  .ddl-copy:disabled { opacity: 0.4; }
  .ddl-loading { padding: var(--s-3) var(--s-4); font-size: 12px; color: var(--muted); }
  .ddl-code { margin: 0; padding: var(--s-3) var(--s-4); max-height: 220px; overflow: auto; font-family: var(--font-mono); font-size: 12px; line-height: 1.55; color: var(--ink-soft); white-space: pre; }

  /* Two stacked panes */
  .pane { overflow: auto; min-height: 0; background: var(--bg-content); }
  .cols-pane { flex: 1.7 1 0; }
  .idx-pane { flex: 1 1 0; border-top: 1px solid var(--border); display: flex; flex-direction: column; }
  .idx-pane .grid { flex: none; }
  .pane-bar { position: sticky; top: 0; z-index: var(--z-sticky); display: flex; align-items: center; gap: var(--s-2); flex: none; padding: var(--s-2) var(--s-4); background: var(--bg-panel); border-bottom: 1px solid var(--border); }
  .pane-title { font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; color: var(--ink-soft); }
  .pane-count { font-size: 11px; color: var(--faint); font-variant-numeric: tabular-nums; }

  /* Grid (columns + indexes share this) */
  .grid { border-collapse: separate; border-spacing: 0; width: max-content; min-width: 100%; }
  .grid th, .grid td {
    border-right: 1px solid var(--hairline); border-bottom: 1px solid var(--hairline);
    padding: 0 var(--s-4); height: 28px; text-align: left; font-size: 12px; white-space: nowrap; user-select: text;
  }
  .grid th { position: sticky; top: 0; z-index: var(--z-sticky); background: var(--bg-panel); color: var(--muted); font-family: var(--font-mono); font-weight: 600; font-size: 11px; border-bottom: 1px solid var(--border); }
  .grid tbody tr:hover td { background: var(--bg-elevated); }

  .gut { width: 44px; min-width: 44px; text-align: right; color: var(--faint); background: var(--bg-panel); user-select: none; font-family: var(--font-mono); font-size: 11px; position: sticky; left: 0; }
  .cn { font-family: var(--font-mono); color: var(--ink); font-weight: 600; }
  .cn-text.editable { cursor: text; }
  .keyi { color: var(--accent); font-size: 10px; margin-right: 5px; }
  .ty { font-family: var(--font-mono); color: var(--accent); }
  .algo { font-family: var(--font-mono); color: var(--muted); font-size: 11px; }
  .fk { font-family: var(--font-mono); color: var(--accent); font-size: 11.5px; }
  .fk-cell { font: inherit; font-family: var(--font-mono); font-size: 11.5px; color: var(--accent); padding: 1px var(--s-2); border-radius: var(--r-xs); text-align: left; }
  .fk-cell:hover { background: var(--bg-elevated); }
  .fk-cell .dim { color: var(--faint); }

  .fk-backdrop { position: fixed; inset: 0; z-index: var(--z-dropdown, 50); }
  .fk-menu { position: fixed; z-index: var(--z-dropdown, 50); min-width: 240px; max-width: 320px; background: var(--bg-elevated); border: 1px solid var(--border-strong); border-radius: 8px; box-shadow: 0 8px 28px rgba(0,0,0,0.5); padding: var(--s-1); display: flex; flex-direction: column; gap: 2px; }
  .fk-existing { padding: var(--s-2) var(--s-3); font-family: var(--font-mono); font-size: 12px; color: var(--accent); }
  .fk-create { display: flex; align-items: center; gap: var(--s-2); text-align: left; padding: var(--s-2) var(--s-3); border-radius: var(--r-sm); font-size: 12.5px; color: var(--ink-soft); }
  .fk-create:hover { background: var(--accent); color: #fff; }
  .fk-create code { font-family: var(--font-mono); }
  .fk-plus { color: var(--accent); font-weight: 700; }
  .fk-create:hover .fk-plus, .fk-create:hover code { color: #fff; }
  .fk-form { display: flex; flex-direction: column; gap: var(--s-2); padding: var(--s-2) var(--s-3); }
  .fk-l { display: flex; flex-direction: column; gap: 3px; font-size: 11px; color: var(--muted); }
  .fk-l select { height: 28px; background: var(--bg-content); border: 1px solid var(--border); border-radius: var(--r-sm); color: var(--ink); font: inherit; font-size: 12.5px; padding: 0 var(--s-2); }
  .fk-l select:focus { outline: none; border-color: var(--accent); }
  .fk-skip { display: inline-flex; align-items: center; gap: 5px; font-size: 11.5px; color: var(--muted); }
  .fk-skip input { accent-color: var(--accent); }
  .fk-err { margin: 0; font-size: 11px; color: var(--danger); white-space: pre-wrap; }
  .fk-tip { margin: 0; font-size: 11px; color: var(--accent); line-height: 1.4; }
  .fk-act { display: flex; justify-content: flex-end; gap: var(--s-2); margin-top: 2px; }
  .mono { font-family: var(--font-mono); color: var(--ink-soft); }
  .dim { color: var(--faint); }
  .mid { text-align: center; padding: var(--s-5) !important; }

  .cellbtn { font: inherit; font-family: var(--font-mono); color: var(--accent); padding: 1px var(--s-2); border-radius: var(--r-xs); text-align: left; }
  .cellbtn:hover:not(:disabled) { background: var(--bg-hover, var(--bg-elevated)); }
  .cellbtn:disabled { opacity: 0.6; }
  .pillbtn { padding: 0; border-radius: var(--r-pill); }
  .pillbtn:hover:not(:disabled) .pill { border-color: var(--accent); color: var(--ink); }
  .pillbtn:disabled { opacity: 0.6; }
  .pill {
    display: inline-block; font-size: 10px; font-weight: 600; letter-spacing: 0.03em;
    padding: 1px 7px; border-radius: var(--r-pill); color: var(--muted); border: 1px solid var(--border-strong);
  }
  .pill.yes { color: var(--faint); }

  .ax { width: 56px; padding: 0 var(--s-2) !important; text-align: right; }
  .act { width: 21px; height: 21px; display: inline-grid; place-items: center; border-radius: var(--r-xs); color: var(--muted); opacity: 0; }
  tr:hover .act { opacity: 1; }
  .act:hover { background: var(--bg-hover, var(--bg-elevated)); color: var(--ink); }
  .act.danger:hover { background: color-mix(in srgb, var(--danger, #e5484d) 18%, transparent); color: var(--danger, #e5484d); }
  .act:disabled { opacity: 0.3 !important; }

  .cin {
    height: 24px; padding: 0 var(--s-2); width: 100%;
    background: var(--bg-content); border: 1px solid var(--border); border-radius: var(--r-xs);
    color: var(--ink); font: inherit; font-size: 12px; font-family: var(--font-mono);
  }
  .cin:focus { outline: none; border-color: var(--accent); }
  .addrow td { background: var(--bg-elevated); }
  .nullbox { display: inline-flex; align-items: center; gap: var(--s-2); font-size: 11px; color: var(--muted); }
  .nullbox input { width: 14px; height: 14px; accent-color: var(--accent); }
  .addactions { display: flex; gap: var(--s-2); align-items: center; }

  /* Add-index form (inside the indexes pane) */
  .idx-form { display: flex; flex-direction: column; gap: var(--s-2); padding: var(--s-3) var(--s-4); flex: none; background: var(--bg-panel); border-bottom: 1px solid var(--hairline); }
  .idx-input { height: 28px; background: var(--bg-content); border: 1px solid var(--border); border-radius: var(--r-sm); color: var(--ink); font: inherit; font-size: 12.5px; padding: 0 var(--s-2); }
  .idx-input:focus { outline: none; border-color: var(--accent); }
  .idx-pick { display: flex; flex-wrap: wrap; gap: var(--s-2) var(--s-3); }
  .idx-chk { display: inline-flex; align-items: center; gap: 4px; font-size: 12px; color: var(--ink-soft); font-family: var(--font-mono); }
  .idx-chk input { accent-color: var(--accent); }
  .idx-actions { display: flex; justify-content: flex-end; gap: var(--s-2); }
  .idx-cancel { font-size: 12px; color: var(--muted); padding: var(--s-1) var(--s-3); border-radius: var(--r-sm); }
  .idx-cancel:hover:not(:disabled) { background: var(--bg-elevated); }
  .idx-save { font-size: 12px; font-weight: 600; color: #fff; background: var(--accent); padding: var(--s-1) var(--s-3); border-radius: var(--r-sm); }
  .idx-save:disabled { opacity: 0.5; }

  .err { padding: var(--s-2) var(--s-4); background: color-mix(in srgb, var(--danger, #e5484d) 12%, transparent); color: var(--danger, #e5484d); font-size: 11.5px; border-bottom: 1px solid var(--hairline); white-space: pre-wrap; flex: none; }
  .err.inline { padding: 0; background: none; border: none; margin: var(--s-3) 0 0; }

  .empty { flex: 1; display: grid; place-items: center; color: var(--faint); font-size: 12.5px; background: var(--bg-content); }

  /* tiny confirm popover */
  .cdrop-backdrop { position: fixed; inset: 0; z-index: var(--z-modal); display: grid; place-items: center; padding: 40px; background: rgba(0,0,0,0.45); }
  .cdrop { width: min(380px, 100%); background: var(--bg-panel); border: 1px solid var(--border-strong); border-radius: var(--r-lg); box-shadow: var(--shadow-modal); padding: var(--s-6) var(--s-7); }
  .cdrop p { margin: 0; font-size: 12.5px; color: var(--ink-soft); line-height: 1.55; }
  .cdrop code { font-family: var(--font-mono); color: var(--ink); background: var(--bg-elevated); padding: 0 4px; border-radius: var(--r-xs); }
  .cdrop-foot { display: flex; gap: var(--s-3); justify-content: flex-end; margin-top: var(--s-5); }

  .btn { height: 30px; padding: 0 var(--s-5); border-radius: var(--r-sm); font: inherit; font-size: 12.5px; font-weight: 600; border: 1px solid transparent; }
  .btn.sm { height: 24px; padding: 0 var(--s-3); font-size: 11.5px; }
  .btn.ghost { background: transparent; border-color: var(--border); color: var(--ink-soft); }
  .btn.ghost:hover { background: var(--bg-elevated); }
  .btn.primary { background: var(--accent); color: var(--accent-ink); }
  .btn.primary:hover:not(:disabled) { filter: brightness(1.05); }
  .btn.primary:disabled { opacity: 0.5; }
  .btn.danger { background: var(--danger, #e5484d); color: #fff; }
  .btn.danger:hover:not(:disabled) { filter: brightness(1.05); }
  .btn.danger:disabled { opacity: 0.5; }
</style>
