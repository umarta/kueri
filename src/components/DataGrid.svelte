<script lang="ts">
  import { createEventDispatcher, tick } from "svelte";
  import { createVirtualizer } from "@tanstack/svelte-virtual";
  import type { QueryResult, RowEdit } from "../lib/types";

  export let result: QueryResult | null = null;
  /** Editing is only safe when the result is a direct single-table browse. */
  export let editable = false;
  /** Currently selected row index (drives the detail panel + highlight). */
  export let selectedRow: number | null = null;
  /** Alternating row background colors (a workspace setting). */
  export let altRows = true;
  /** Current sort (drives the header indicator); set by the parent. */
  export let sort: { col: string; dir: "asc" | "desc" } | null = null;
  /** Whether clicking a header sorts (table browse only). */
  export let sortable = false;
  /** `schema.table` for persisting per-table column visibility ("" = no persist). */
  export let tableKey = "";

  const dispatch = createEventDispatcher<{
    commit: RowEdit[];
    selectRow: number;
    sortColumn: string;
    deleteRows: number[];
  }>();

  // ── Multi-row selection (for copy / delete) ─────────────────────────────────
  let selected = new Set<number>();
  let anchor = -1;

  function rowClick(i: number, e: MouseEvent) {
    if (e.shiftKey && anchor >= 0) {
      const [a, b] = anchor < i ? [anchor, i] : [i, anchor];
      const s = new Set(selected);
      for (let k = a; k <= b; k++) s.add(k);
      selected = s;
    } else if (e.metaKey || e.ctrlKey) {
      const s = new Set(selected);
      if (s.has(i)) s.delete(i);
      else s.add(i);
      selected = s;
      anchor = i;
    } else {
      selected = new Set([i]);
      anchor = i;
    }
    dispatch("selectRow", i);
  }
  function clearSel() {
    selected = new Set();
  }
  async function copySelected() {
    if (!result || !selected.size) return;
    const idx = [...selected].sort((a, b) => a - b);
    const text = idx.map((i) => result!.rows[i].map(fmt).join("\t")).join("\n");
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      /* clipboard unavailable */
    }
  }
  function requestDelete() {
    if (selected.size) dispatch("deleteRows", [...selected].sort((a, b) => a - b));
  }

  const GUTTER = 48;
  const ROW_H = 28;

  // ── Staged edits ──────────────────────────────────────────────────────────
  let edits: Record<string, string> = {};
  let editing: { r: number; c: number } | null = null;
  let draft = "";
  let prev: QueryResult | null = null;
  let input: HTMLInputElement;

  $: if (result !== prev) {
    prev = result;
    edits = {};
    editing = null;
    selected = new Set();
    anchor = -1;
  }
  $: pending = Object.keys(edits);

  const key = (r: number, c: number) => `${r}:${c}`;
  const isEdited = (r: number, c: number) => key(r, c) in edits;

  function fmt(v: unknown): string {
    if (v === null || v === undefined) return "NULL";
    if (typeof v === "object") return JSON.stringify(v);
    return String(v);
  }
  const isNull = (v: unknown) => v === null || v === undefined;
  const display = (r: number, c: number, raw: unknown) =>
    key(r, c) in edits ? edits[key(r, c)] : fmt(raw);

  async function startEdit(r: number, c: number, raw: unknown) {
    if (!editable) return;
    editing = { r, c };
    const k = key(r, c);
    draft = k in edits ? edits[k] : isNull(raw) ? "" : fmt(raw);
    await tick();
    input?.focus();
    input?.select();
  }

  function commitCell() {
    if (!editing || !result) return;
    const { r, c } = editing;
    const k = key(r, c);
    const orig = result.rows[r][c];
    const origStr = isNull(orig) ? "" : fmt(orig);
    if (draft === origStr) delete edits[k];
    else edits[k] = draft;
    edits = edits;
    editing = null;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Enter") { e.preventDefault(); commitCell(); }
    else if (e.key === "Escape") { e.preventDefault(); editing = null; }
  }

  function discard() { edits = {}; editing = null; }

  /** Commit edits (⌘S). Flush the cell being edited first so a typed-but-not-
   *  Entered value is included; then push staged edits. No-op if nothing changed. */
  export function commitStaged() {
    if (editing) commitCell();
    if (Object.keys(edits).length) commit();
  }

  function commit() {
    if (!result) return;
    const byRow: Record<number, RowEdit> = {};
    for (const k of Object.keys(edits)) {
      const [r, c] = k.split(":").map(Number);
      byRow[r] ??= { rowIndex: r, original: result.rows[r], updates: {} };
      byRow[r].updates[result.columns[c]] = edits[k];
    }
    dispatch("commit", Object.values(byRow));
  }

  // ── Column widths (mono ≈ 7.3px/char), sampled from header + first 60 rows ──
  function widthOf(res: QueryResult, c: number): number {
    let max = res.columns[c].length;
    const n = Math.min(res.rows.length, 60);
    for (let i = 0; i < n; i++) {
      const len = fmt(res.rows[i][c]).length;
      if (len > max) max = len;
    }
    return Math.min(460, Math.max(84, Math.round(max * 7.3 + 26)));
  }
  // Columns the user has hidden, persisted per table.
  let hidden = new Set<string>();
  let colMenu: { right: number; top: number } | null = null;
  $: loadHidden(tableKey);
  function loadHidden(k: string) {
    try {
      const raw = k ? localStorage.getItem("kueri.cols." + k) : null;
      hidden = new Set(raw ? (JSON.parse(raw) as string[]) : []);
    } catch {
      hidden = new Set();
    }
  }
  function saveHidden() {
    if (!tableKey) return;
    try {
      localStorage.setItem("kueri.cols." + tableKey, JSON.stringify([...hidden]));
    } catch {
      /* storage unavailable */
    }
  }
  function toggleCol(name: string) {
    const s = new Set(hidden);
    if (s.has(name)) s.delete(name);
    else s.add(name);
    hidden = s;
    saveHidden();
  }
  function showAllCols() {
    hidden = new Set();
    saveHidden();
  }
  function openColMenu(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    colMenu = colMenu ? null : { right: window.innerWidth - r.right, top: r.bottom + 4 };
  }

  // Visible columns (with their original index), and widths/template over them.
  $: visible = result ? result.columns.map((name, i) => ({ name, i })).filter((v) => !hidden.has(v.name)) : [];
  $: vwidths = result ? visible.map((v) => widthOf(result!, v.i)) : [];
  $: template = `${GUTTER}px ${vwidths.map((w) => `${w}px`).join(" ")}`;
  $: totalWidth = GUTTER + vwidths.reduce((a, b) => a + b, 0);

  // ── Row virtualization ──────────────────────────────────────────────────────
  let scrollEl: HTMLDivElement | undefined;
  $: rowCount = result?.rows.length ?? 0;
  // Pass scrollEl + rowCount as args so the store is rebuilt when either changes
  // (binding the scroll element, or loading a new result).
  $: virtualizer = makeVirtualizer(scrollEl, rowCount);

  function makeVirtualizer(_el: HTMLDivElement | undefined, count: number) {
    return createVirtualizer<HTMLDivElement, HTMLDivElement>({
      count,
      getScrollElement: () => scrollEl ?? null,
      estimateSize: () => ROW_H,
      overscan: 14,
    });
  }

  // Re-runs on every virtualizer emission (scroll, measure, count change).
  $: vtotal = $virtualizer.getTotalSize();
  $: vitems = $virtualizer.getVirtualItems();
</script>

{#if result}
  <div class="meta">
    <span class="count">{result.row_count.toLocaleString()} {result.row_count === 1 ? "row" : "rows"}</span>
    <span class="cols">{result.columns.length} columns</span>
    <div class="meta-spacer"></div>
    {#if editable}<span class="editable-hint">double-click a cell to edit</span>{/if}
    {#if result.columns.length}
      <button class="cols-btn" on:click={openColMenu} title="Show / hide columns">
        Columns{#if hidden.size} · {hidden.size} hidden{/if}
      </button>
    {/if}
  </div>

  <div class="wrap" bind:this={scrollEl}>
    <div class="surface" role="grid" aria-rowcount={result.row_count} style="width: {totalWidth}px">
      <div class="head" role="row" style="grid-template-columns: {template}">
        <div class="hcell gutter" role="columnheader"></div>
        {#each visible as v (v.name)}
          <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
          <div class="hcell" class:sortable role="columnheader" tabindex="-1" title={v.name} on:click={() => sortable && dispatch("sortColumn", v.name)}>
            <span class="hname">{v.name}</span>
            {#if sort && sort.col === v.name}<span class="sortind">{sort.dir === "asc" ? "↑" : "↓"}</span>{/if}
          </div>
        {/each}
      </div>

      <div class="body" style="height: {vtotal}px">
          {#each vitems as vrow (vrow.index)}
            {@const i = vrow.index}
            <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
            <div
              class="row"
              role="row"
              tabindex="-1"
              aria-rowindex={i + 1}
              class:alt={altRows && i % 2 === 1}
              class:selected={selected.has(i) || selectedRow === i}
              on:click={(e) => rowClick(i, e)}
              style="transform: translateY({vrow.start}px); height: {ROW_H}px; grid-template-columns: {template}"
            >
              <div class="cell gutter" role="gridcell">{i + 1}</div>
              {#each visible as v (v.name)}
                {@const j = v.i}
                {@const cell = result.rows[i][j]}
                <div
                  class="cell"
                  role="gridcell"
                  tabindex="-1"
                  class:null={isNull(cell) && !isEdited(i, j)}
                  class:edited={isEdited(i, j)}
                  class:active={editing?.r === i && editing?.c === j}
                  class:can-edit={editable}
                  title={display(i, j, cell)}
                  on:dblclick={() => startEdit(i, j, cell)}
                  on:keydown={(e) => { if (editable && (e.key === "Enter" || e.key === "F2")) { e.preventDefault(); startEdit(i, j, cell); } }}
                >
                  {#if editing?.r === i && editing?.c === j}
                    <input class="cell-input" bind:this={input} bind:value={draft} on:keydown={onKey} on:blur={commitCell} spellcheck="false" />
                  {:else}
                    {display(i, j, cell)}
                  {/if}
                </div>
              {/each}
            </div>
          {/each}
      </div>
    </div>
    {#if result.rows.length === 0}<div class="no-rows">Query returned no rows.</div>{/if}
  </div>

  {#if colMenu}
    <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
    <div class="menu-backdrop" on:click={() => (colMenu = null)}></div>
    <div class="col-menu" style="right: {colMenu.right}px; top: {colMenu.top}px;">
      <button class="col-all" on:click={showAllCols}>Show all</button>
      <div class="cm-sep"></div>
      {#each result.columns as name (name)}
        <label class="col-item">
          <input type="checkbox" checked={!hidden.has(name)} on:change={() => toggleCol(name)} />
          <span>{name}</span>
        </label>
      {/each}
    </div>
  {/if}

  {#if selected.size}
    <div class="commit-bar sel-bar" role="status">
      <span class="badge sel">{selected.size}</span>
      <span class="ctext">selected</span>
      <div class="spacer"></div>
      <button class="btn" on:click={copySelected}>Copy</button>
      {#if editable}<button class="btn btn-danger" on:click={requestDelete}>Delete</button>{/if}
      <button class="btn" on:click={clearSel}>Clear</button>
    </div>
  {/if}

  {#if pending.length}
    <div class="commit-bar" role="status">
      <span class="badge">{pending.length}</span>
      <span class="ctext">unsaved {pending.length === 1 ? "change" : "changes"}</span>
      <div class="spacer"></div>
      <button class="btn" on:click={discard}>Discard</button>
      <button class="btn btn-primary" on:click={commit}>Commit</button>
    </div>
  {/if}
{:else}
  <div class="empty">
    <svg viewBox="0 0 48 48" width="40" height="40" aria-hidden="true">
      <rect x="8" y="10" width="32" height="28" rx="3" fill="none" stroke="currentColor" stroke-width="1.6"/>
      <line x1="8" y1="18" x2="40" y2="18" stroke="currentColor" stroke-width="1.6"/>
      <line x1="18" y1="18" x2="18" y2="38" stroke="currentColor" stroke-width="1.6"/>
      <line x1="8" y1="28" x2="40" y2="28" stroke="currentColor" stroke-width="1.6"/>
    </svg>
    <p class="e-title">No results yet</p>
    <p class="e-sub">Pick a table on the left, or write SQL above and press ⌘↵.</p>
  </div>
{/if}

<style>
  .meta {
    display: flex; align-items: center; gap: var(--s-5);
    padding: var(--s-2) var(--s-5); background: var(--bg-panel);
    border-bottom: 1px solid var(--hairline); flex: none;
  }
  .count { font-size: 11.5px; font-weight: 600; color: var(--ink-soft); }
  .cols { font-size: 11.5px; color: var(--faint); }
  .meta-spacer { flex: 1; }
  .editable-hint { font-size: 11px; color: var(--faint); }
  .cols-btn { font-size: 11.5px; color: var(--muted); padding: 2px var(--s-2); border-radius: var(--r-xs); }
  .cols-btn:hover { background: var(--bg-elevated); color: var(--ink); }

  .menu-backdrop { position: fixed; inset: 0; z-index: var(--z-dropdown); }
  .col-menu {
    position: fixed; z-index: var(--z-dropdown); min-width: 180px; max-height: 360px; overflow-y: auto;
    background: var(--bg-elevated); border: 1px solid var(--border-strong);
    border-radius: var(--r-md); box-shadow: var(--shadow-pop); padding: var(--s-1);
    display: flex; flex-direction: column;
  }
  .col-all { text-align: left; padding: var(--s-2) var(--s-3); border-radius: var(--r-sm); font-size: 12px; color: var(--accent); }
  .col-all:hover { background: var(--bg-panel); }
  .cm-sep { height: 1px; margin: var(--s-1) var(--s-2); background: var(--hairline); }
  .col-item { display: flex; align-items: center; gap: var(--s-2); padding: var(--s-2) var(--s-3); border-radius: var(--r-sm); font-size: 12.5px; color: var(--ink-soft); font-family: var(--font-mono); cursor: pointer; }
  .col-item:hover { background: var(--bg-panel); }
  .col-item input { accent-color: var(--accent); }

  .wrap { flex: 1; overflow: auto; background: var(--bg-content); position: relative; }
  .surface { min-width: 100%; }

  /* Header — sticky to the top of the scroll container */
  .head {
    display: grid; position: sticky; top: 0; z-index: var(--z-sticky);
    background: var(--bg-panel); border-bottom: 1px solid var(--border);
  }
  .hcell {
    height: 28px; display: flex; align-items: center; gap: 4px; padding: 0 var(--s-4);
    font-size: 11.5px; font-weight: 600; color: var(--ink-soft);
    border-right: 1px solid var(--hairline);
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .hcell.sortable { cursor: pointer; }
  .hcell.sortable:hover { color: var(--ink); background: var(--bg-elevated); }
  .hname { overflow: hidden; text-overflow: ellipsis; }
  .sortind { margin-left: auto; color: var(--accent); font-size: 11px; flex: none; }

  .body { position: relative; width: 100%; }
  .row {
    display: grid; position: absolute; top: 0; left: 0; width: 100%;
    border-bottom: 1px solid var(--hairline);
  }
  .row.alt .cell { background: rgba(255, 255, 255, 0.018); }
  .row:hover .cell { background: var(--bg-elevated); }
  .row.selected .cell { background: color-mix(in srgb, var(--accent) 20%, transparent); }
  .row.selected .gutter { background: color-mix(in srgb, var(--accent) 26%, transparent); color: var(--ink); }

  .cell {
    display: flex; align-items: center; padding: 0 var(--s-4);
    font-family: var(--font-mono); font-size: 12px;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    border-right: 1px solid var(--hairline); user-select: text;
  }
  .cell.can-edit { cursor: cell; }
  .cell.null { color: var(--faint); font-style: italic; }

  /* Staged edit — tinted, not a side-stripe */
  .cell.edited {
    background: rgba(255, 214, 10, 0.12); color: var(--warn);
    box-shadow: inset 0 0 0 1px rgba(255, 214, 10, 0.35);
  }
  .row:hover .cell.edited { background: rgba(255, 214, 10, 0.16); }
  .cell.active { padding: 0; box-shadow: inset 0 0 0 2px var(--accent); }
  .cell-input {
    width: 100%; height: 100%; padding: 0 var(--s-4);
    border: none; outline: none; background: var(--bg-content); color: var(--ink);
    font-family: var(--font-mono); font-size: 12px;
  }

  /* Row-number gutter — sticky to the left edge */
  .gutter {
    position: sticky; left: 0;
    width: 48px; min-width: 48px;
    justify-content: flex-end; color: var(--faint);
    background: var(--bg-panel); user-select: none; font-size: 11px;
  }
  .row:hover .gutter { background: var(--bg-elevated); color: var(--muted); }
  .head .gutter { z-index: 1; }       /* corner sits above sticky body gutter */
  .row .gutter { z-index: 1; }

  .no-rows { padding: var(--s-8); text-align: center; color: var(--muted); font-size: 12.5px; }

  /* Commit bar */
  .commit-bar {
    display: flex; align-items: center; gap: var(--s-3);
    padding: var(--s-3) var(--s-5); flex: none;
    background: var(--bg-panel); border-top: 1px solid var(--border);
    animation: slideUp var(--t-base) var(--ease-out);
  }
  .badge {
    min-width: 20px; height: 20px; padding: 0 6px; border-radius: var(--r-pill);
    display: inline-grid; place-items: center; font-size: 11px; font-weight: 700;
    background: var(--warn); color: #1a1500;
  }
  .ctext { font-size: 12px; color: var(--ink-soft); }
  .spacer { flex: 1; }
  .badge.sel { background: var(--accent); color: var(--accent-ink); }
  .sel-bar { animation: none; }
  .btn-danger { background: var(--danger); color: #fff; border-color: transparent; }
  .btn-danger:hover { filter: brightness(1.05); }
  @keyframes slideUp { from { transform: translateY(100%); opacity: 0; } }
  @media (prefers-reduced-motion: reduce) { .commit-bar { animation: none; } }

  .empty {
    flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center;
    gap: var(--s-3); color: var(--faint); background: var(--bg-content);
  }
  .empty svg { color: var(--border-strong); }
  .e-title { margin: var(--s-2) 0 0; font-size: 13.5px; font-weight: 600; color: var(--muted); }
  .e-sub { margin: 0; font-size: 12px; color: var(--faint); }
</style>
