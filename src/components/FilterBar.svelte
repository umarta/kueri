<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { FilterCond, FilterOp } from "../lib/types";

  export let columns: string[] = [];
  export let filters: FilterCond[] = [];

  const dispatch = createEventDispatcher<{ apply: FilterCond[]; clear: void }>();

  const OPS: { value: FilterOp; label: string; noValue?: boolean }[] = [
    { value: "=", label: "=" },
    { value: "!=", label: "≠" },
    { value: ">", label: ">" },
    { value: "<", label: "<" },
    { value: ">=", label: "≥" },
    { value: "<=", label: "≤" },
    { value: "contains", label: "contains" },
    { value: "starts", label: "starts with" },
    { value: "is null", label: "is null", noValue: true },
    { value: "is not null", label: "is not null", noValue: true },
  ];
  const noVal = (op: FilterOp) => OPS.find((o) => o.value === op)?.noValue ?? false;

  // Local working copy; seed with one empty row if none.
  let rows: FilterCond[] = filters.length
    ? filters.map((f) => ({ ...f }))
    : [{ column: columns[0] ?? "", op: "=", value: "" }];

  function addRow() {
    rows = [...rows, { column: columns[0] ?? "", op: "=", value: "" }];
  }
  function removeRow(i: number) {
    rows = rows.filter((_, idx) => idx !== i);
    if (rows.length === 0) rows = [{ column: columns[0] ?? "", op: "=", value: "" }];
  }
  function apply() {
    dispatch(
      "apply",
      rows.filter((r) => r.column && (noVal(r.op) || r.value !== "")),
    );
  }
  function clear() {
    rows = [{ column: columns[0] ?? "", op: "=", value: "" }];
    dispatch("clear");
  }
</script>

<div class="filterbar">
  {#each rows as r, i (i)}
    <div class="frow">
      <select class="fc col" bind:value={r.column}>
        {#each columns as c (c)}<option value={c}>{c}</option>{/each}
      </select>
      <select class="fc op" bind:value={r.op}>
        {#each OPS as o (o.value)}<option value={o.value}>{o.label}</option>{/each}
      </select>
      <input
        class="fc val"
        bind:value={r.value}
        placeholder={noVal(r.op) ? "—" : "value"}
        disabled={noVal(r.op)}
        spellcheck="false"
        on:keydown={(e) => { if (e.key === "Enter") apply(); }}
      />
      <button class="fx" title="Remove" on:click={() => removeRow(i)}>
        <svg viewBox="0 0 12 12" width="11" height="11" aria-hidden="true"><path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg>
      </button>
      {#if i === rows.length - 1}
        <button class="fadd" title="Add condition" on:click={addRow}>
          <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true"><path d="M8 3.5v9M3.5 8h9" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
        </button>
      {/if}
    </div>
  {/each}
  <div class="spacer"></div>
  <button class="btn ghost" on:click={clear}>Clear</button>
  <button class="btn primary" on:click={apply}>Apply</button>
</div>

<style>
  .filterbar {
    display: flex; align-items: center; flex-wrap: wrap; gap: var(--s-2);
    padding: var(--s-2) var(--s-4); background: var(--bg-panel);
    border-bottom: 1px solid var(--hairline); flex: none;
  }
  .frow { display: inline-flex; align-items: center; gap: var(--s-1); }
  .fc { height: 26px; background: var(--bg-content); border: 1px solid var(--border); border-radius: var(--r-sm); color: var(--ink); font: inherit; font-size: 12px; padding: 0 var(--s-2); }
  .fc:focus { outline: none; border-color: var(--accent); }
  .col { max-width: 160px; font-family: var(--font-mono); }
  .op { width: 92px; }
  .val { width: 150px; }
  .val:disabled { opacity: 0.5; }
  .fx, .fadd { width: 24px; height: 26px; display: grid; place-items: center; border-radius: var(--r-sm); color: var(--faint); border: 1px solid transparent; }
  .fx:hover { color: var(--danger, #e5484d); background: var(--bg-elevated); }
  .fadd { color: var(--accent); }
  .fadd:hover { background: var(--bg-elevated); }
  .spacer { flex: 1; }
  .btn { height: 26px; padding: 0 var(--s-4); border-radius: var(--r-sm); font: inherit; font-size: 12px; font-weight: 600; border: 1px solid transparent; }
  .btn.ghost { background: transparent; border-color: var(--border); color: var(--ink-soft); }
  .btn.ghost:hover { background: var(--bg-elevated); }
  .btn.primary { background: var(--accent); color: var(--accent-ink); }
  .btn.primary:hover { filter: brightness(1.05); }
</style>
