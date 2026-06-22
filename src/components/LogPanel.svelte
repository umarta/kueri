<script lang="ts">
  import { afterUpdate, createEventDispatcher } from "svelte";
  import { queryLog, clearLog } from "../lib/stores/log";

  const dispatch = createEventDispatcher<{ close: void }>();

  let scrollEl: HTMLDivElement;
  let pinned = true;

  // Keep the view pinned to the newest entry unless the user scrolled up.
  afterUpdate(() => {
    if (pinned && scrollEl) scrollEl.scrollTop = scrollEl.scrollHeight;
  });
  function onScroll() {
    if (!scrollEl) return;
    pinned = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight < 24;
  }
</script>

<section class="log">
  <header class="lhead">
    <span class="ltitle">Query Log</span>
    <span class="lcount">{$queryLog.length}</span>
    <div class="spacer"></div>
    <button class="lbtn" on:click={clearLog} title="Clear log">Clear</button>
    <button class="lbtn icon" on:click={() => dispatch("close")} title="Hide log" aria-label="Hide log">
      <svg viewBox="0 0 14 14" width="12" height="12" aria-hidden="true"><path d="M3.5 3.5l7 7M10.5 3.5l-7 7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg>
    </button>
  </header>

  <div class="lbody" bind:this={scrollEl} on:scroll={onScroll}>
    {#if $queryLog.length === 0}
      <div class="lempty">No queries yet.</div>
    {:else}
      {#each $queryLog as e (e.id)}
        <div class="lrow" class:err={e.error}>
          <span class="ltime">{e.time}</span>
          {#if e.ms !== undefined}<span class="lms">{e.ms}ms</span>{/if}
          <code class="lsql">{e.sql}</code>
          {#if e.error}<span class="lerr">{e.error}</span>{/if}
        </div>
      {/each}
    {/if}
  </div>
</section>

<style>
  .log { display: flex; flex-direction: column; height: 100%; background: var(--bg-content); border-top: 1px solid var(--border); min-height: 0; }
  .lhead { display: flex; align-items: center; gap: var(--s-2); padding: var(--s-2) var(--s-4); background: var(--bg-panel); border-bottom: 1px solid var(--hairline); flex: none; }
  .ltitle { font-size: 11.5px; font-weight: 600; color: var(--ink-soft); }
  .lcount { font-size: 10px; color: var(--faint); background: var(--bg-elevated); border-radius: 999px; padding: 0 6px; line-height: 16px; }
  .spacer { flex: 1; }
  .lbtn { font-size: 11px; color: var(--muted); padding: 2px var(--s-2); border-radius: var(--r-xs); }
  .lbtn:hover { background: var(--bg-elevated); color: var(--ink); }
  .lbtn.icon { display: grid; place-items: center; width: 22px; height: 20px; padding: 0; }

  .lbody { flex: 1; overflow-y: auto; padding: var(--s-2) 0; font-family: var(--font-mono); font-size: 11.5px; }
  .lempty { padding: var(--s-5); color: var(--faint); text-align: center; }
  .lrow { display: flex; align-items: baseline; gap: var(--s-3); padding: 2px var(--s-4); white-space: pre-wrap; word-break: break-word; }
  .lrow:hover { background: var(--bg-panel); }
  .ltime { color: var(--faint); flex: none; }
  .lms { color: var(--ok, #3fb950); flex: none; font-size: 10.5px; }
  .lsql { color: var(--ink-soft); }
  .lrow.err .lsql { color: var(--danger, #e5484d); }
  .lerr { color: var(--danger, #e5484d); }
  /**set log body 100% width*/
  .log { width: 100%; }
</style>
