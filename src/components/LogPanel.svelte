<script lang="ts">
  import { afterUpdate, createEventDispatcher } from "svelte";
  import { queryLog, clearLog } from "../lib/stores/log";

  const dispatch = createEventDispatcher<{ close: void; pick: string }>();

  let scrollEl: HTMLDivElement;
  let pinned = true;
  let q = "";

  $: shown = q.trim()
    ? $queryLog.filter((e) => e.sql.toLowerCase().includes(q.trim().toLowerCase()))
    : $queryLog;

  // Keep the view pinned to the newest entry unless searching or scrolled up.
  afterUpdate(() => {
    if (pinned && !q && scrollEl) scrollEl.scrollTop = scrollEl.scrollHeight;
  });
  function onScroll() {
    if (!scrollEl) return;
    pinned = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight < 24;
  }
</script>

<section class="log">
  <header class="lhead">
    <span class="ltitle">Query History</span>
    <span class="lcount">{q ? `${shown.length}/${$queryLog.length}` : $queryLog.length}</span>
    <div class="lsearch">
      <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true"><circle cx="7" cy="7" r="4.5" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M11 11l3 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
      <input bind:value={q} placeholder="Search history…" spellcheck="false" />
      {#if q}<button class="lclear" on:click={() => (q = "")} aria-label="Clear search">✕</button>{/if}
    </div>
    <div class="spacer"></div>
    <button class="lbtn" on:click={clearLog} title="Clear history">Clear</button>
    <button class="lbtn icon" on:click={() => dispatch("close")} title="Hide history" aria-label="Hide history">
      <svg viewBox="0 0 14 14" width="12" height="12" aria-hidden="true"><path d="M3.5 3.5l7 7M10.5 3.5l-7 7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg>
    </button>
  </header>

  <div class="lbody" bind:this={scrollEl} on:scroll={onScroll}>
    {#if shown.length === 0}
      <div class="lempty">{q ? "No matches." : "No queries yet."}</div>
    {:else}
      {#each shown as e (e.id)}
        <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
        <div class="lrow" class:err={e.error} title="Click to load into a new query tab" on:click={() => dispatch("pick", e.sql)}>
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
  .log { display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-content); border-top: 1px solid var(--border); min-height: 0; }
  .lhead { display: flex; align-items: center; gap: var(--s-2); padding: var(--s-2) var(--s-4); background: var(--bg-panel); border-bottom: 1px solid var(--hairline); flex: none; }
  .ltitle { font-size: 11.5px; font-weight: 600; color: var(--ink-soft); }
  .lcount { font-size: 10px; color: var(--faint); background: var(--bg-elevated); border-radius: 999px; padding: 0 6px; line-height: 16px; font-variant-numeric: tabular-nums; }
  .lsearch { display: flex; align-items: center; gap: var(--s-2); width: 220px; height: 24px; margin-left: var(--s-2); padding: 0 var(--s-2); background: var(--bg-content); border: 1px solid var(--border); border-radius: var(--r-sm); color: var(--faint); }
  .lsearch input { flex: 1; min-width: 0; background: none; border: none; outline: none; color: var(--ink); font: inherit; font-size: 12px; }
  .lclear { color: var(--faint); font-size: 11px; padding: 0 2px; }
  .lclear:hover { color: var(--ink); }
  .spacer { flex: 1; }
  .lbtn { font-size: 11px; color: var(--muted); padding: 2px var(--s-2); border-radius: var(--r-xs); }
  .lbtn:hover { background: var(--bg-elevated); color: var(--ink); }
  .lbtn.icon { display: grid; place-items: center; width: 22px; height: 20px; padding: 0; }

  .lbody { flex: 1; overflow-y: auto; padding: var(--s-2) 0; font-family: var(--font-mono); font-size: 11.5px; }
  .lempty { padding: var(--s-5); color: var(--faint); text-align: center; }
  .lrow { display: flex; align-items: baseline; gap: var(--s-3); padding: 2px var(--s-4); white-space: pre-wrap; word-break: break-word; cursor: pointer; }
  .lrow:hover { background: var(--bg-panel); }
  .ltime { color: var(--faint); flex: none; }
  .lms { color: var(--ok, #3fb950); flex: none; font-size: 10.5px; }
  .lsql { color: var(--ink-soft); }
  .lrow.err .lsql { color: var(--danger, #e5484d); }
  .lerr { color: var(--danger, #e5484d); }
</style>
