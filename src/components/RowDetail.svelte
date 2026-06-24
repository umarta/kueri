<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { DateInput } from "date-picker-svelte";
  import {
    isDate, isDateTime, isDateTimeTz, toDateValue, toDateString, toDateOnlyString,
    splitTz, combineTz, TZ_OFFSETS,
  } from "../lib/datetime";
  import type { QueryResult, RowEdit, ColumnInfo } from "../lib/types";

  export let result: QueryResult | null = null;
  export let index: number | null = null;
  export let columns: ColumnInfo[] = [];
  export let editable = false;
  /** Insert mode: a blank form built from `columns` for a new row. */
  export let insert = false;
  /** Seed values for insert mode (used by Duplicate Row). */
  export let initial: Record<string, string | null> | null = null;
  /** Bumped by the parent each time insert/duplicate opens, to re-seed the form. */
  export let insertNonce = 0;

  const dispatch = createEventDispatcher<{
    commit: RowEdit[];
    insert: Record<string, string | null>;
    close: void;
  }>();

  let edits: Record<string, string | null> = {};
  let key = "";
  let menu: { col: string; i: number; right: number; top: number; json: boolean } | null = null;

  // Reset when the target changes (row selection, or entering/leaving insert mode).
  $: rowKey = insert ? `insert:${insertNonce}` : `${result?.columns.length ?? 0}:${index}`;
  $: if (rowKey !== key) {
    key = rowKey;
    edits = insert && initial ? { ...initial } : {};
    menu = null;
  }

  $: row = result && index !== null ? result.rows[index] : null;
  $: typeMap = new Map(columns.map((c) => [c.name, c.data_type]));
  $: enumMap = new Map(columns.map((c) => [c.name, c.enum_values ?? []]));

  type Entry = { col: string; type: string; i: number };
  $: entries = (insert
    ? columns.map((c, i) => ({ col: c.name, type: c.data_type, i }))
    : result
      ? result.columns.map((col, i) => ({ col, type: typeMap.get(col) ?? "", i }))
      : []) as Entry[];

  const isBool = (t: string) => /bool/i.test(t);
  const isJson = (t: string) => /json/i.test(t);
  const isInteger = (t: string) => /\b(int|integer|smallint|bigint|tinyint|mediumint|serial|oid)\b/i.test(t);
  const isNumeric = (t: string) => /\b(decimal|numeric|float|double|real|money)\b/i.test(t);
  const isNull = (v: unknown) => v === null || v === undefined;
  function fmt(v: unknown): string {
    if (v === null || v === undefined) return "";
    if (typeof v === "object") return JSON.stringify(v);
    return String(v);
  }
  const prettyJson = (s: string) => { try { return JSON.stringify(JSON.parse(s), null, 2); } catch { return s; } };
  const minifyJson = (s: string) => { try { return JSON.stringify(JSON.parse(s)); } catch { return s; } };

  const NULLK = "\0NULL";
  const orig = (e: Entry) => (insert ? "" : isNull(row?.[e.i]) ? NULLK : fmt(row?.[e.i]));
  const curStr = (e: Entry) => (e.col in edits ? edits[e.col] ?? "" : insert ? "" : fmt(row?.[e.i]));
  const nulled = (e: Entry) => (e.col in edits ? edits[e.col] === null : insert ? false : isNull(row?.[e.i]));
  const provided = (e: Entry) => e.col in edits; // insert: was this column given a value?
  const isEnum = (e: Entry) => (enumMap.get(e.col)?.length ?? 0) > 0;
  // Options for an enum select; keep the current value even if it's not in the set.
  function enumOpts(e: Entry): string[] {
    const base = enumMap.get(e.col) ?? [];
    if (!insert && !nulled(e)) {
      const cur = curStr(e);
      if (cur && !base.includes(cur)) return [cur, ...base];
    }
    return base;
  }

  function setVal(e: Entry, v: string | null) {
    if (insert) {
      if (v === "") delete edits[e.col];
      else edits[e.col] = v;
    } else {
      const nk = v === null ? NULLK : v;
      if (nk === orig(e)) delete edits[e.col];
      else edits[e.col] = v;
    }
    edits = edits;
  }
  function unset(e: Entry) {
    delete edits[e.col];
    edits = edits;
  }
  const jsonDisplay = (e: Entry) =>
    e.col in edits ? edits[e.col] ?? "" : insert ? "" : prettyJson(fmt(row?.[e.i]));

  function openMenu(e: Entry, ev: MouseEvent) {
    const r = (ev.currentTarget as HTMLElement).getBoundingClientRect();
    menu = { col: e.col, i: e.i, right: window.innerWidth - r.right, top: r.bottom + 4, json: isJson(e.type) };
  }
  const menuEntry = (): Entry | undefined => entries.find((x) => x.col === menu?.col);

  $: dirty = Object.keys(edits).length;
  $: hasForm = insert ? columns.length > 0 : !!(result && row);

  function save() {
    if (!result || index === null || !dirty) return;
    dispatch("commit", [{ rowIndex: index, original: result.rows[index], updates: { ...edits } }]);
  }
  function doInsert() {
    dispatch("insert", { ...edits });
  }
</script>

<svelte:window on:keydown={(e) => { if (e.key === "Escape") menu = null; }} />

<aside class="detail" aria-label="Row detail">
  <header class="head">
    <span class="title">{insert ? "New row" : index !== null ? `Row ${index + 1}` : "Row"}</span>
    <button class="close" on:click={() => dispatch("close")} title="Close" aria-label="Close">
      <svg viewBox="0 0 14 14" width="13" height="13" aria-hidden="true"><path d="M3.5 3.5l7 7M10.5 3.5l-7 7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg>
    </button>
  </header>

  {#if hasForm}
    <div class="fields">
      {#each entries as e (e.col)}
        <div class="rd-field">
          <div class="fhead">
            <span class="fname">{e.col}</span>
            {#if e.type}<span class="ftype">{e.type}</span>{/if}
          </div>

          {#if insert || editable}
            <div class="rd-control" class:edited={e.col in edits} class:nulled={nulled(e)}>
              {#if isEnum(e)}
                <select
                  class="rd-input rd-select"
                  aria-label={e.col}
                  on:change={(ev) => {
                    const v = ev.currentTarget.value;
                    if (v === "__rd_default__") unset(e);
                    else setVal(e, v === "__rd_null__" ? null : v);
                  }}
                >
                  {#if insert}<option value="__rd_default__" selected={!provided(e)}>(default)</option>{/if}
                  {#each enumOpts(e) as opt (opt)}
                    <option value={opt} selected={curStr(e) === opt && !nulled(e) && (!insert || provided(e))}>{opt}</option>
                  {/each}
                  <option value="__rd_null__" selected={nulled(e)}>NULL</option>
                </select>
              {:else if isBool(e.type)}
                <select
                  class="rd-input rd-select"
                  aria-label={e.col}
                  on:change={(ev) => {
                    const v = ev.currentTarget.value;
                    if (v === "__rd_default__") unset(e);
                    else setVal(e, v === "__rd_null__" ? null : v);
                  }}
                >
                  {#if insert}<option value="__rd_default__" selected={!provided(e)}>(default)</option>{/if}
                  <option value="true" selected={curStr(e) === "true" && !nulled(e) && (!insert || provided(e))}>true</option>
                  <option value="false" selected={curStr(e) === "false" && !nulled(e) && (!insert || provided(e))}>false</option>
                  <option value="__rd_null__" selected={nulled(e)}>NULL</option>
                </select>
              {:else if isJson(e.type)}
                <textarea
                  class="rd-input rd-textarea"
                  class:nullph={nulled(e)}
                  aria-label={e.col}
                  rows="6"
                  spellcheck="false"
                  placeholder={nulled(e) ? "NULL" : insert ? "DEFAULT" : ""}
                  value={nulled(e) ? "" : jsonDisplay(e)}
                  on:input={(ev) => setVal(e, ev.currentTarget.value)}
                ></textarea>
                <button class="rd-menu-btn top" title="Field options" aria-label="Field options" on:click={(ev) => openMenu(e, ev)}>⋯</button>
              {:else if isDate(e.type)}
                <DateInput
                  class="rd-input"
                  value={insert && !provided(e) ? new Date() : toDateValue(curStr(e))}
                  format="yyyy-MM-dd"
                  dynamicPositioning
                  placeholder={nulled(e) ? "NULL" : insert ? "DEFAULT" : "2020-12-31"}
                  on:select={(ev) => setVal(e, toDateOnlyString(ev.detail))}
                />
                <button class="rd-menu-btn" title="Field options" aria-label="Field options" on:click={(ev) => openMenu(e, ev)}>⋯</button>
              {:else if isDateTimeTz(e.type)}
                {@const tz = splitTz(e.col in edits ? edits[e.col] : insert ? "" : fmt(row?.[e.i]))}
                <div class="rd-tzrow">
                  <DateInput
                    class="rd-input"
                    value={tz.wall ? toDateValue(tz.wall) : insert && !provided(e) ? new Date() : null}
                    format="yyyy-MM-dd HH:mm:ss"
                    timePrecision="second"
                    dynamicPositioning
                    placeholder={nulled(e) ? "NULL" : insert ? "DEFAULT" : "2020-12-31 23:59:59"}
                    on:select={(ev) => setVal(e, combineTz(toDateString(ev.detail) ?? "", tz.offset))}
                  />
                  <select
                    class="rd-input rd-select rd-tzsel"
                    aria-label="Time zone offset"
                    value={tz.offset}
                    on:change={(ev) => setVal(e, combineTz(tz.wall || (toDateString(new Date()) ?? ""), ev.currentTarget.value))}
                  >
                    {#each TZ_OFFSETS as o (o)}<option value={o}>UTC{o}</option>{/each}
                  </select>
                </div>
                <button class="rd-menu-btn" title="Field options" aria-label="Field options" on:click={(ev) => openMenu(e, ev)}>⋯</button>
              {:else if isDateTime(e.type)}
                <DateInput
                  class="rd-input"
                  value={insert && !provided(e) ? new Date() : toDateValue(curStr(e))}
                  format="yyyy-MM-dd HH:mm:ss"
                  timePrecision="second"
                  dynamicPositioning
                  placeholder={nulled(e) ? "NULL" : insert ? "DEFAULT" : "2020-12-31 23:59:59"}
                  on:select={(ev) => setVal(e, toDateString(ev.detail))}
                />
                <button class="rd-menu-btn" title="Field options" aria-label="Field options" on:click={(ev) => openMenu(e, ev)}>⋯</button>
              {:else if isInteger(e.type)}
                <input
                  class="rd-input"
                  type="number"
                  inputmode="numeric"
                  step="1"
                  class:nullph={nulled(e)}
                  aria-label={e.col}
                  placeholder={nulled(e) ? "NULL" : insert ? "DEFAULT" : "0"}
                  value={nulled(e) ? "" : curStr(e)}
                  on:input={(ev) => setVal(e, ev.currentTarget.value)}
                />
                <button class="rd-menu-btn" title="Field options" aria-label="Field options" on:click={(ev) => openMenu(e, ev)}>⋯</button>
              {:else if isNumeric(e.type)}
                <input
                  class="rd-input"
                  type="number"
                  inputmode="decimal"
                  step="any"
                  class:nullph={nulled(e)}
                  aria-label={e.col}
                  placeholder={nulled(e) ? "NULL" : insert ? "DEFAULT" : "0.0"}
                  value={nulled(e) ? "" : curStr(e)}
                  on:input={(ev) => setVal(e, ev.currentTarget.value)}
                />
                <button class="rd-menu-btn" title="Field options" aria-label="Field options" on:click={(ev) => openMenu(e, ev)}>⋯</button>
              {:else}
                <input
                  class="rd-input"
                  class:nullph={nulled(e)}
                  aria-label={e.col}
                  spellcheck="false"
                  placeholder={nulled(e) ? "NULL" : insert ? "DEFAULT" : ""}
                  value={nulled(e) ? "" : curStr(e)}
                  on:input={(ev) => setVal(e, ev.currentTarget.value)}
                />
                <button class="rd-menu-btn" title="Field options" aria-label="Field options" on:click={(ev) => openMenu(e, ev)}>⋯</button>
              {/if}
            </div>
          {:else}
            <div class="fval ro" class:nullv={isNull(row?.[e.i])}>{isNull(row?.[e.i]) ? "NULL" : isJson(e.type) ? prettyJson(fmt(row?.[e.i])) : fmt(row?.[e.i])}</div>
          {/if}
        </div>
      {/each}
    </div>

    {#if insert}
      <footer class="foot">
        <span class="dirty">{dirty ? `${dirty} value${dirty === 1 ? "" : "s"} set` : "all defaults"}</span>
        <button class="save-btn" on:click={doInsert}>Insert</button>
      </footer>
    {:else if editable}
      <footer class="foot">
        <span class="dirty">{dirty ? `${dirty} changed` : "No changes"}</span>
        <button class="save-btn" on:click={save} disabled={!dirty}>Save</button>
      </footer>
    {/if}
  {:else}
    <div class="empty">Select a row to see its values.</div>
  {/if}
</aside>

{#if menu}
  {@const m = menu}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="menu-backdrop" on:click={() => (menu = null)}></div>
  <div class="rd-menu" style="right: {m.right}px; top: {m.top}px;">
    <button on:click={() => { const e = menuEntry(); if (e) setVal(e, null); menu = null; }}>Set NULL</button>
    <button on:click={() => { const e = menuEntry(); if (e) setVal(e, ""); menu = null; }}>Set empty</button>
    {#if m.json}
      <div class="sep"></div>
      <button on:click={() => { const e = menuEntry(); if (e) setVal(e, prettyJson(curStr(e))); menu = null; }}>Pretty</button>
      <button on:click={() => { const e = menuEntry(); if (e) setVal(e, minifyJson(curStr(e))); menu = null; }}>Minify</button>
    {/if}
  </div>
{/if}

<style>
  .detail { display: flex; flex-direction: column; background: var(--bg-panel); border-left: 1px solid var(--border); min-width: 0; flex: 1; overflow: hidden; }

  .head { display: flex; align-items: center; justify-content: space-between; padding: var(--s-3) var(--s-4); border-bottom: 1px solid var(--hairline); flex: none; }
  .title { font-size: 12px; font-weight: 600; color: var(--ink); }
  .close { width: 24px; height: 24px; display: grid; place-items: center; border-radius: var(--r-sm); color: var(--muted); }
  .close:hover { background: var(--bg-elevated); color: var(--ink); }

  .fields { flex: 1; overflow-y: auto; padding: var(--s-3) var(--s-4); }

  .rd-field { margin-bottom: var(--s-5); }
  .fhead { display: flex; align-items: baseline; justify-content: space-between; gap: var(--s-3); margin-bottom: var(--s-2); }
  .fname { font-size: 11.5px; font-weight: 600; color: var(--muted); font-family: var(--font-mono); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ftype { font-size: 10px; color: var(--faint); font-family: var(--font-mono); flex: none; }

  .fval { display: block; font-size: 12.5px; color: var(--ink); font-family: var(--font-mono); word-break: break-word; }
  .fval.ro { user-select: text; white-space: pre-wrap; min-height: 18px; padding: var(--s-1) 0; }
  .nullv { color: var(--faint); font-style: italic; }

  .rd-control { position: relative; }
  .rd-input {
    appearance: none; -webkit-appearance: none;
    width: 100%; margin: 0; box-sizing: border-box;
    background: var(--bg-content); border: 1px solid var(--border); border-radius: var(--r-sm);
    color: var(--ink); font-family: var(--font-mono); font-size: 12.5px;
    transition: border-color var(--t-fast) var(--ease-out);
  }
  .rd-input:focus { outline: none; border-color: var(--accent); }
  input.rd-input { height: 30px; padding: 0 26px 0 var(--s-3); }
  .rd-select { height: 30px; padding: 0 var(--s-3); cursor: pointer;
    background-image: url("data:image/svg+xml,%3Csvg width='10' height='6' xmlns='http://www.w3.org/2000/svg'%3E%3Cpath d='M1 1l4 4 4-4' stroke='%239494a0' stroke-width='1.5' fill='none' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
    background-repeat: no-repeat; background-position: right var(--s-3) center; padding-right: var(--s-7); }
  .rd-textarea { padding: var(--s-2) var(--s-3); min-height: 64px; line-height: 1.5; resize: vertical; white-space: pre; }
  .nullph::placeholder { color: var(--faint); font-style: italic; }

  .rd-control.edited .rd-input { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 10%, var(--bg-content)); }

  .rd-menu-btn {
    position: absolute; right: 4px; top: 50%; transform: translateY(-50%);
    width: 18px; height: 22px; border-radius: var(--r-xs); display: grid; place-items: center;
    color: var(--faint); font-size: 13px; line-height: 1;
  }
  .rd-menu-btn.top { top: 5px; transform: none; }
  .rd-menu-btn:hover { color: var(--ink); background: var(--bg-elevated); }

  /* timestamp-with-time-zone: picker + UTC-offset selector */
  .rd-tzrow { display: flex; gap: var(--s-2); align-items: center; padding-right: 22px; }
  .rd-tzrow :global(.date-time-field) { flex: 1; min-width: 0; }
  .rd-tzsel { flex: none; width: 104px; height: 30px; }

  .menu-backdrop { position: fixed; inset: 0; z-index: var(--z-dropdown); }
  .rd-menu {
    position: fixed; z-index: var(--z-dropdown); min-width: 140px;
    background: var(--bg-elevated); border: 1px solid var(--border-strong);
    border-radius: var(--r-md); box-shadow: var(--shadow-pop); padding: var(--s-1);
    display: flex; flex-direction: column;
  }
  .rd-menu button { text-align: left; padding: var(--s-2) var(--s-3); border-radius: var(--r-sm); font-size: 12.5px; color: var(--ink-soft); }
  .rd-menu button:hover { background: var(--accent); color: var(--accent-ink); }
  .rd-menu .sep { height: 1px; margin: var(--s-1) var(--s-2); background: var(--hairline); }

  .foot { display: flex; align-items: center; justify-content: space-between; gap: var(--s-3); padding: var(--s-3) var(--s-4); border-top: 1px solid var(--hairline); flex: none; }
  .dirty { font-size: 11px; color: var(--faint); }
  .save-btn { height: 28px; padding: 0 var(--s-5); border-radius: var(--r-sm); font: inherit; font-size: 12px; font-weight: 600; border: 1px solid transparent; background: var(--accent); color: var(--accent-ink); }
  .save-btn:hover:not(:disabled) { filter: brightness(1.05); }
  .save-btn:disabled { opacity: 0.5; }

  .empty { flex: 1; display: grid; place-items: center; color: var(--faint); font-size: 12px; padding: var(--s-5); text-align: center; }
</style>
