<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { QueryTab } from "../lib/types";

  export let tabs: QueryTab[] = [];
  export let activeId = "";

  const dispatch = createEventDispatcher<{ select: string; close: string; new: void }>();
</script>

<div class="tabstrip" role="tablist">
  {#each tabs as t (t.id)}
    <div class="tab" class:active={t.id === activeId} role="tab" aria-selected={t.id === activeId}>
      <button class="label" on:click={() => dispatch("select", t.id)} title={t.title}>
        {#if t.running}
          <span class="dot run"></span>
        {:else if t.kind === "table"}
          <svg class="ticon" viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
            <rect x="2.5" y="3" width="11" height="10" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.2"/>
            <line x1="2.5" y1="6.3" x2="13.5" y2="6.3" stroke="currentColor" stroke-width="1.2"/>
            <line x1="6.5" y1="6.3" x2="6.5" y2="13" stroke="currentColor" stroke-width="1.2"/>
          </svg>
        {:else}
          <svg class="ticon" viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
            <path d="M4 5l3 3-3 3M8.5 11h4" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        {/if}
        <span class="txt">{t.title}</span>
      </button>
      {#if tabs.length > 1}
        <button class="close" title="Close tab (⌘W)" aria-label="Close tab" on:click={() => dispatch("close", t.id)}>
          <svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true"><path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
        </button>
      {/if}
    </div>
  {/each}
  <button class="add" title="New query tab (⌘T)" aria-label="New query tab" on:click={() => dispatch("new")}>
    <svg viewBox="0 0 14 14" width="13" height="13" aria-hidden="true"><path d="M7 3v8M3 7h8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
  </button>
</div>

<style>
  .tabstrip {
    display: flex; align-items: stretch; gap: 2px;
    height: 36px; padding: var(--s-2) var(--s-2) 0;
    background: var(--bg-app); border-bottom: 1px solid var(--border);
    overflow-x: auto; flex: none;
  }
  .tabstrip::-webkit-scrollbar { height: 0; }

  .tab {
    display: flex; align-items: center; min-width: 0; max-width: 220px;
    border-radius: var(--r-sm) var(--r-sm) 0 0;
    color: var(--muted);
    transition: background var(--t-fast) var(--ease-out), color var(--t-fast) var(--ease-out);
  }
  .tab:hover { background: var(--bg-panel); color: var(--ink-soft); }
  .tab.active { background: var(--bg-panel); color: var(--ink); box-shadow: inset 0 -1px 0 var(--bg-panel); }

  .label {
    display: flex; align-items: center; gap: var(--s-2); min-width: 0;
    padding: 0 var(--s-2) 0 var(--s-4); height: 28px; color: inherit;
  }
  .txt { font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .ticon { color: var(--faint); flex: none; }
  .tab.active .ticon { color: var(--muted); }
  .dot { width: 6px; height: 6px; border-radius: 50%; flex: none; background: transparent; }
  .dot.run { background: var(--accent); animation: pulse 1s var(--ease-out) infinite; }

  .close {
    width: 18px; height: 18px; margin-right: var(--s-2); border-radius: var(--r-xs);
    display: grid; place-items: center; color: var(--faint); flex: none;
    opacity: 0; transition: opacity var(--t-fast) var(--ease-out), background var(--t-fast) var(--ease-out);
  }
  .tab:hover .close, .tab.active .close { opacity: 1; }
  .close:hover { background: var(--bg-active); color: var(--ink); }

  .add {
    width: 28px; height: 28px; margin: 0 var(--s-1); align-self: center; border-radius: var(--r-sm);
    display: grid; place-items: center; color: var(--muted); flex: none;
    transition: background var(--t-fast) var(--ease-out), color var(--t-fast) var(--ease-out);
  }
  .add:hover { background: var(--bg-panel); color: var(--ink); }

  @keyframes pulse { 50% { opacity: 0.35; } }
  @media (prefers-reduced-motion: reduce) { .dot.run { animation: none; } }
</style>
