<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import ExplainNode from "./ExplainNode.svelte";

  export let root: Record<string, unknown>;
  export let sql = "";

  const dispatch = createEventDispatcher<{ close: void }>();
  $: totalCost = root["Total Cost"];
</script>

<!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
<div class="backdrop" on:click|self={() => dispatch("close")}>
  <div class="panel" role="dialog" aria-label="Query plan">
    <header class="head">
      <span class="title">Query plan</span>
      {#if totalCost !== undefined}<span class="tcost">total cost {Number(totalCost).toLocaleString()}</span>{/if}
      <button class="x" on:click={() => dispatch("close")} aria-label="Close">✕</button>
    </header>
    {#if sql}<pre class="sql">{sql}</pre>{/if}
    <div class="tree">
      <ExplainNode node={root} />
    </div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; z-index: var(--z-modal); display: grid; place-items: start center; padding: 7vh 32px; background: rgba(0, 0, 0, 0.45); }
  .panel { width: min(780px, 96vw); max-height: 82vh; display: flex; flex-direction: column; background: var(--bg-panel); border: 1px solid var(--border-strong); border-radius: var(--r-lg); box-shadow: var(--shadow-modal); overflow: hidden; }
  .head { display: flex; align-items: center; gap: var(--s-3); padding: var(--s-3) var(--s-4); border-bottom: 1px solid var(--hairline); flex: none; }
  .title { font-size: 13px; font-weight: 600; color: var(--ink); }
  .tcost { font-size: 11.5px; color: var(--muted); font-family: var(--font-mono); }
  .x { margin-left: auto; width: 26px; height: 26px; border-radius: var(--r-sm); color: var(--muted); }
  .x:hover { background: var(--bg-elevated); color: var(--ink); }
  .sql { margin: 0; padding: var(--s-3) var(--s-4); border-bottom: 1px solid var(--hairline); background: var(--bg-content); color: var(--ink-soft); font-family: var(--font-mono); font-size: 11.5px; white-space: pre-wrap; max-height: 96px; overflow: auto; flex: none; }
  .tree { padding: var(--s-3) var(--s-4); overflow: auto; }
</style>
