<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { api } from "../lib/tauri";
  import { typeOptions, alterTypeOptions, supportsColumnAlter, type ColumnDraft } from "../lib/ddl";
  import { readOnly } from "../lib/stores/connection";
  import type { ColumnInfo, DbKind } from "../lib/types";

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
  <div class="meta">
    <span class="count">{schema && table ? `${schema}.${table}` : table}</span>
    <span class="cols">{columns.length} columns</span>
    {#if canEdit}
      <button class="addbtn" on:click={startAdd} disabled={busy}>
        <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true"><path d="M8 3.5v9M3.5 8h9" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
        Add column
      </button>
    {/if}
  </div>

  {#if error}<div class="err">{error}</div>{/if}

  <div class="wrap">
    <table class="struct">
      <thead>
        <tr><th class="gutter"></th><th>Column</th><th>Type</th><th>Nullable</th><th>Default</th><th class="acth"></th></tr>
      </thead>
      <tbody>
        {#each columns as c, i (c.name)}
          <tr>
            <td class="gutter">{i + 1}</td>
            <td class="name">
              {#if renaming === c.name}
                <!-- svelte-ignore a11y-autofocus -->
                <input
                  class="rename"
                  bind:value={renameVal}
                  autofocus
                  spellcheck="false"
                  on:keydown={(e) => { if (e.key === "Enter") commitRename(); else if (e.key === "Escape") renaming = null; }}
                  on:blur={commitRename}
                />
              {:else}
                {c.name}
              {/if}
            </td>
            <td class="type">
              {#if typingCol === c.name}
                <!-- svelte-ignore a11y-autofocus -->
                <select class="cin" bind:value={typeVal} autofocus on:change={() => commitType(c)} on:blur={() => (typingCol = null)}>
                  {#each alterTypeOptions(kind) as opt (opt)}<option value={opt}>{opt}</option>{/each}
                  {#if !alterTypeOptions(kind).includes(c.data_type)}<option value={c.data_type}>{c.data_type}</option>{/if}
                </select>
              {:else if alterOk}
                <button class="cellbtn" on:click={() => startType(c)} disabled={busy} title="Change type">{c.data_type}</button>
              {:else}
                {c.data_type}
              {/if}
            </td>
            <td>
              {#if alterOk}
                <button class="pillbtn" on:click={() => toggleNull(c)} disabled={busy} title="Toggle nullability">
                  <span class="pill" class:yes={c.nullable}>{c.nullable ? "NULL" : "NOT NULL"}</span>
                </button>
              {:else}
                <span class="pill" class:yes={c.nullable}>{c.nullable ? "NULL" : "NOT NULL"}</span>
              {/if}
            </td>
            <td class:null={c.default === null}>{c.default ?? "—"}</td>
            <td class="actc">
              {#if canEdit}
                <button class="act" title="Rename column" on:click={() => startRename(c.name)} disabled={busy}>
                  <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true"><path d="M10.5 2.5l3 3L6 13l-3.5.5L3 10z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/></svg>
                </button>
                <button class="act danger" title="Drop column" on:click={() => (confirmDrop = c.name)} disabled={busy}>
                  <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true"><path d="M3 4.5h10M6.5 4V2.8h3V4M5 4.5l.5 8.5h5l.5-8.5" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>
                </button>
              {/if}
            </td>
          </tr>
        {/each}

        {#if adding}
          <tr class="addrow">
            <td class="gutter">+</td>
            <td><input class="cin" bind:value={draft.name} placeholder="column_name" spellcheck="false"
              on:keydown={(e) => { if (e.key === "Enter") addColumn(); else if (e.key === "Escape") adding = false; }} /></td>
            <td>
              <select class="cin" bind:value={draft.type}>
                {#each typeOptions(kind) as opt (opt)}<option value={opt}>{opt}</option>{/each}
              </select>
            </td>
            <td><label class="nullbox"><input type="checkbox" bind:checked={draft.nullable} /> NULL</label></td>
            <td class="addactions" colspan="2">
              <button class="btn primary sm" on:click={addColumn} disabled={busy || !draft.name.trim()}>Add</button>
              <button class="btn ghost sm" on:click={() => (adding = false)} disabled={busy}>Cancel</button>
            </td>
          </tr>
        {/if}
      </tbody>
    </table>
  </div>
{:else}
  <div class="empty">Select a table to inspect its structure.</div>
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
  .meta { display: flex; align-items: center; gap: var(--s-5); padding: var(--s-2) var(--s-5); background: var(--bg-panel); border-bottom: 1px solid var(--hairline); flex: none; }
  .count { font-size: 11.5px; font-weight: 600; color: var(--ink-soft); font-family: var(--font-mono); }
  .cols { font-size: 11.5px; color: var(--faint); }
  .addbtn { margin-left: auto; display: inline-flex; align-items: center; gap: var(--s-2); padding: var(--s-2) var(--s-3); border-radius: var(--r-sm); font-size: 11.5px; font-weight: 600; color: var(--accent); }
  .addbtn:hover:not(:disabled) { background: var(--bg-elevated); }
  .addbtn:disabled { opacity: 0.5; }

  .err { padding: var(--s-2) var(--s-5); background: color-mix(in srgb, var(--danger, #e5484d) 12%, transparent); color: var(--danger, #e5484d); font-size: 11.5px; border-bottom: 1px solid var(--hairline); white-space: pre-wrap; }
  .err.inline { padding: 0; background: none; border: none; margin: var(--s-3) 0 0; }

  .wrap { flex: 1; overflow: auto; background: var(--bg-content); }
  .struct { border-collapse: separate; border-spacing: 0; width: max-content; min-width: 100%; }
  .struct th, .struct td {
    border-right: 1px solid var(--hairline); border-bottom: 1px solid var(--hairline);
    padding: 0 var(--s-5); height: 28px; text-align: left; font-size: 12px; white-space: nowrap; user-select: text;
  }
  .struct th { position: sticky; top: 0; z-index: var(--z-sticky); background: var(--bg-panel); color: var(--ink-soft); font-weight: 600; font-size: 11.5px; border-bottom: 1px solid var(--border); }
  .struct tbody tr:hover td { background: var(--bg-elevated); }
  .name { font-family: var(--font-mono); color: var(--ink); font-weight: 600; }
  .type { font-family: var(--font-mono); color: var(--accent); }
  .null { color: var(--faint); font-style: italic; }

  .cellbtn { font: inherit; font-family: var(--font-mono); color: var(--accent); padding: 1px var(--s-2); border-radius: var(--r-xs); text-align: left; }
  .cellbtn:hover:not(:disabled) { background: var(--bg-hover); }
  .cellbtn:disabled { opacity: 0.6; }
  .pillbtn { padding: 0; border-radius: var(--r-pill); }
  .pillbtn:hover:not(:disabled) .pill { border-color: var(--accent); color: var(--ink); }
  .pillbtn:disabled { opacity: 0.6; }

  .gutter { width: 48px; min-width: 48px; text-align: right; color: var(--faint); background: var(--bg-panel); user-select: none; font-family: var(--font-mono); font-size: 11px; position: sticky; left: 0; }

  .pill {
    display: inline-block; font-size: 10px; font-weight: 600; letter-spacing: 0.03em;
    padding: 1px 7px; border-radius: var(--r-pill); color: var(--muted);
    border: 1px solid var(--border-strong);
  }
  .pill.yes { color: var(--faint); }

  .acth { width: 64px; }
  .actc { width: 64px; padding: 0 var(--s-3) !important; }
  .act { width: 22px; height: 22px; display: inline-grid; place-items: center; border-radius: var(--r-xs); color: var(--muted); opacity: 0; }
  tr:hover .act { opacity: 1; }
  .act:hover { background: var(--bg-hover); color: var(--ink); }
  .act.danger:hover { background: color-mix(in srgb, var(--danger, #e5484d) 18%, transparent); color: var(--danger, #e5484d); }
  .act:disabled { opacity: 0.3 !important; }

  .rename, .cin {
    height: 24px; padding: 0 var(--s-2); width: 100%;
    background: var(--bg-content); border: 1px solid var(--accent); border-radius: var(--r-xs);
    color: var(--ink); font: inherit; font-size: 12px; font-family: var(--font-mono);
  }
  .cin { border-color: var(--border); }
  .cin:focus, .rename:focus { outline: none; border-color: var(--accent); }
  .addrow td { background: var(--bg-elevated); }
  .nullbox { display: inline-flex; align-items: center; gap: var(--s-2); font-size: 11px; color: var(--muted); }
  .nullbox input { width: 14px; height: 14px; accent-color: var(--accent); }
  .addactions { display: flex; gap: var(--s-2); align-items: center; }

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
