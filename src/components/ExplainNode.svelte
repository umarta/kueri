<script lang="ts">
  // One node in a Postgres EXPLAIN plan tree (recurses via <svelte:self>).
  type Plan = Record<string, unknown>;
  export let node: Plan;
  export let depth = 0;

  $: children = (node["Plans"] as Plan[] | undefined) ?? [];
  const n = (v: unknown): string =>
    typeof v === "number" ? (Number.isInteger(v) ? v.toLocaleString() : v.toFixed(2)) : String(v ?? "");
  const conds: [string, string][] = [
    ["Index Cond", "Index Cond"],
    ["Recheck Cond", "Recheck"],
    ["Hash Cond", "Hash Cond"],
    ["Merge Cond", "Merge Cond"],
    ["Join Filter", "Join Filter"],
    ["Filter", "Filter"],
    ["Sort Key", "Sort Key"],
  ];
  const txt = (v: unknown) => (Array.isArray(v) ? v.join(", ") : String(v));
  $: actual = node["Actual Total Time"] !== undefined;
</script>

<div class="enode" style="--depth: {depth}">
  <div class="erow">
    {#if depth > 0}<span class="ebranch" aria-hidden="true">└─</span>{/if}
    <span class="etype">{node["Node Type"]}{#if node["Join Type"]} · {node["Join Type"]}{/if}{#if node["Strategy"] && node["Strategy"] !== "Plain"} · {node["Strategy"]}{/if}</span>
    {#if node["Relation Name"]}<span class="erel">on {node["Relation Name"]}{#if node["Alias"] && node["Alias"] !== node["Relation Name"]} {node["Alias"]}{/if}</span>
    {:else if node["Index Name"]}<span class="erel">using {node["Index Name"]}</span>{/if}
    <span class="estat">
      cost <b>{n(node["Total Cost"])}</b> · rows {n(node["Plan Rows"])}
      {#if actual}· <span class="eact">{n(node["Actual Total Time"])} ms · {n(node["Actual Rows"])} rows</span>{/if}
    </span>
  </div>
  {#each conds as [k, label]}
    {#if node[k]}<div class="econd"><span class="eclabel">{label}</span> {txt(node[k])}</div>{/if}
  {/each}
</div>

{#each children as child (child)}
  <svelte:self node={child} depth={depth + 1} />
{/each}

<style>
  .enode { padding-left: calc(var(--depth) * 18px); }
  .erow { display: flex; align-items: baseline; gap: var(--s-2); padding: 3px 0; flex-wrap: wrap; }
  .ebranch { color: var(--faint); font-family: var(--font-mono); font-size: 11px; }
  .etype { font-weight: 700; color: var(--ink); font-size: 12.5px; }
  .erel { color: var(--accent); font-family: var(--font-mono); font-size: 12px; }
  .estat { margin-left: auto; color: var(--muted); font-size: 11.5px; font-family: var(--font-mono); white-space: nowrap; }
  .estat b { color: var(--ink-soft); font-weight: 700; }
  .eact { color: var(--ok, #18a558); }
  .econd { padding-left: 22px; color: var(--faint); font-size: 11px; font-family: var(--font-mono); }
  .eclabel { color: var(--muted); }
</style>
