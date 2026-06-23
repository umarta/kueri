<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
  import { api } from "../lib/tauri";
  import type { ColumnInfo } from "../lib/types";

  export let columns: ColumnInfo[] = [];
  export let schema = "";
  export let table = "";

  const dispatch = createEventDispatcher<{
    import: { columns: string[]; rows: string[][] };
    close: void;
  }>();

  let loading = true;
  let error = "";
  let fileName = "";
  let rawRows: string[][] = [];
  let hasHeader = true;
  // target column name -> CSV column index (-1 = skip)
  let mapping: Record<string, number> = {};

  $: csvCols =
    rawRows.length === 0
      ? []
      : hasHeader
        ? rawRows[0]
        : rawRows[0].map((_, i) => `Column ${i + 1}`);
  $: dataRows = hasHeader ? rawRows.slice(1) : rawRows;
  $: mappedCols = columns.filter((c) => (mapping[c.name] ?? -1) >= 0);
  $: preview = dataRows.slice(0, 5);

  onMount(pickFile);

  async function pickFile() {
    try {
      const path = await openFileDialog({
        multiple: false,
        filters: [{ name: "CSV", extensions: ["csv", "tsv", "txt"] }],
      });
      if (!path || Array.isArray(path)) {
        dispatch("close");
        return;
      }
      fileName = String(path).split(/[\\/]/).pop() ?? String(path);
      const text = await api.readTextFile(String(path));
      rawRows = parseCsv(text);
      if (rawRows.length === 0) {
        error = "The file is empty.";
      }
      defaultMapping();
    } catch (e) {
      error = (e as { message?: string })?.message ?? String(e);
    } finally {
      loading = false;
    }
  }

  function defaultMapping() {
    const m: Record<string, number> = {};
    const cols = csvCols;
    for (const c of columns) {
      const i = cols.findIndex((h) => h.trim().toLowerCase() === c.name.toLowerCase());
      m[c.name] = i; // -1 when no header match
    }
    mapping = m;
  }
  // Re-derive defaults when the header toggle flips the column labels.
  let lastHeader = hasHeader;
  $: if (rawRows.length && hasHeader !== lastHeader) {
    lastHeader = hasHeader;
    defaultMapping();
  }

  function parseCsv(text: string): string[][] {
    const rows: string[][] = [];
    let row: string[] = [];
    let cur = "";
    let q = false;
    for (let i = 0; i < text.length; i++) {
      const c = text[i];
      if (q) {
        if (c === '"') {
          if (text[i + 1] === '"') { cur += '"'; i++; }
          else q = false;
        } else cur += c;
      } else if (c === '"') q = true;
      else if (c === ",") { row.push(cur); cur = ""; }
      else if (c === "\n") { row.push(cur); rows.push(row); row = []; cur = ""; }
      else if (c !== "\r") cur += c;
    }
    if (cur !== "" || row.length) { row.push(cur); rows.push(row); }
    return rows;
  }

  function doImport() {
    if (!mappedCols.length) return;
    const idx = mappedCols.map((c) => mapping[c.name]);
    const rows = dataRows.map((r) => idx.map((i) => r[i] ?? ""));
    dispatch("import", { columns: mappedCols.map((c) => c.name), rows });
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
<div class="backdrop" on:click|self={() => dispatch("close")}>
  <div class="dialog" role="dialog" aria-modal="true">
    <header class="dhead">
      <span class="dtitle">Import CSV → <code>{schema}.{table}</code></span>
      <button class="x" on:click={() => dispatch("close")} aria-label="Close">✕</button>
    </header>

    {#if loading}
      <div class="dbody"><p class="muted">Reading file…</p></div>
    {:else if error}
      <div class="dbody"><p class="err">{error}</p></div>
    {:else}
      <div class="dbody">
        <div class="meta">
          <span class="fname">{fileName}</span>
          <span class="muted">{dataRows.length} rows · {csvCols.length} columns</span>
          <label class="chk"><input type="checkbox" bind:checked={hasHeader} /> First row is header</label>
        </div>

        <div class="map">
          <div class="maprow maphead"><span>Table column</span><span>CSV column</span></div>
          {#each columns as c (c.name)}
            <div class="maprow">
              <span class="tcol"><code>{c.name}</code><span class="dt">{c.data_type}</span></span>
              <select bind:value={mapping[c.name]}>
                <option value={-1}>— skip —</option>
                {#each csvCols as h, i (i)}<option value={i}>{h || `Column ${i + 1}`}</option>{/each}
              </select>
            </div>
          {/each}
        </div>

        {#if mappedCols.length && preview.length}
          <div class="prev">
            <div class="muted prevlbl">Preview (first {preview.length})</div>
            <div class="ptable">
              <div class="prow phead">{#each mappedCols as c (c.name)}<span>{c.name}</span>{/each}</div>
              {#each preview as r (r)}
                <div class="prow">{#each mappedCols as c (c.name)}<span class="pcell">{r[mapping[c.name]] ?? ""}</span>{/each}</div>
              {/each}
            </div>
          </div>
        {/if}
      </div>

      <footer class="dfoot">
        <span class="muted">{mappedCols.length} column{mappedCols.length === 1 ? "" : "s"} mapped</span>
        <div class="fspace"></div>
        <button class="btn ghost" on:click={() => dispatch("close")}>Cancel</button>
        <button class="btn primary" on:click={doImport} disabled={!mappedCols.length || !dataRows.length}>
          Import {dataRows.length} rows
        </button>
      </footer>
    {/if}
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; z-index: var(--z-modal); display: grid; place-items: center; padding: 40px; background: rgba(0,0,0,0.45); }
  .dialog { width: min(620px, 100%); max-height: 80vh; display: flex; flex-direction: column; background: var(--bg-panel); border: 1px solid var(--border-strong); border-radius: var(--r-lg); box-shadow: var(--shadow-modal); overflow: hidden; }
  .dhead { display: flex; align-items: center; justify-content: space-between; padding: var(--s-4) var(--s-5); border-bottom: 1px solid var(--hairline); flex: none; }
  .dtitle { font-size: 13px; font-weight: 600; color: var(--ink); }
  .dtitle code, .tcol code { font-family: var(--font-mono); color: var(--accent); }
  .x { width: 24px; height: 24px; border-radius: var(--r-sm); color: var(--muted); }
  .x:hover { background: var(--bg-elevated); color: var(--ink); }

  .dbody { padding: var(--s-4) var(--s-5); overflow-y: auto; }
  .muted { color: var(--muted); font-size: 12px; }
  .err { color: var(--danger); font-size: 12.5px; white-space: pre-wrap; }
  .meta { display: flex; align-items: center; gap: var(--s-4); margin-bottom: var(--s-4); flex-wrap: wrap; }
  .fname { font-family: var(--font-mono); font-size: 12px; color: var(--ink-soft); }
  .chk { display: inline-flex; align-items: center; gap: var(--s-2); font-size: 12px; color: var(--ink-soft); margin-left: auto; }
  .chk input { accent-color: var(--accent); }

  .map { display: flex; flex-direction: column; gap: 4px; }
  .maprow { display: grid; grid-template-columns: 1fr 1fr; gap: var(--s-3); align-items: center; }
  .maphead { font-size: 10.5px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--faint); margin-bottom: 2px; }
  .tcol { display: flex; align-items: baseline; gap: var(--s-2); overflow: hidden; }
  .dt { font-size: 10px; color: var(--faint); font-family: var(--font-mono); }
  .maprow select { height: 28px; background: var(--bg-content); border: 1px solid var(--border); border-radius: var(--r-sm); color: var(--ink); font: inherit; font-size: 12px; padding: 0 var(--s-2); }
  .maprow select:focus { outline: none; border-color: var(--accent); }

  .prev { margin-top: var(--s-4); }
  .prevlbl { margin-bottom: var(--s-2); }
  .ptable { border: 1px solid var(--hairline); border-radius: var(--r-sm); overflow-x: auto; font-family: var(--font-mono); font-size: 11px; }
  .prow { display: flex; }
  .prow.phead { background: var(--bg-elevated); color: var(--muted); font-weight: 600; }
  .prow span { flex: 1; min-width: 90px; padding: 3px var(--s-3); border-right: 1px solid var(--hairline); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .pcell { color: var(--ink-soft); }

  .dfoot { display: flex; align-items: center; gap: var(--s-3); padding: var(--s-3) var(--s-5); border-top: 1px solid var(--hairline); flex: none; }
  .fspace { flex: 1; }
  .btn { height: 30px; padding: 0 var(--s-5); border-radius: var(--r-sm); font: inherit; font-size: 12.5px; font-weight: 600; border: 1px solid transparent; }
  .btn.ghost { background: transparent; border-color: var(--border); color: var(--ink-soft); }
  .btn.ghost:hover { background: var(--bg-elevated); }
  .btn.primary { background: var(--accent); color: var(--accent-ink); }
  .btn.primary:hover:not(:disabled) { filter: brightness(1.05); }
  .btn.primary:disabled { opacity: 0.5; }
</style>
