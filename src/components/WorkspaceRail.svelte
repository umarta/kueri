<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { dbKind, statusVar } from "../lib/dbKinds";
  import type { Workspace } from "../lib/stores/connection";

  export let workspaces: Workspace[] = [];
  export let activeId: string | null = null;

  const dispatch = createEventDispatcher<{ switch: string; add: void; close: string }>();
</script>

<nav class="rail" aria-label="Connections">
  <div class="items">
    {#each workspaces as w (w.id)}
      {@const meta = dbKind(w.config.kind)}
      <div class="slot" class:active={w.id === activeId}>
        <button
          class="ws"
          title={`${w.config.name}${w.config.database ? " · " + w.config.database : ""}`}
          on:click={() => dispatch("switch", w.id)}
        >
          {#if w.id === activeId}<span class="bar" aria-hidden="true"></span>{/if}
          <span class="badge" style="--c: {meta.color}">
            {meta.abbr}
            <span class="status" style="--s: {statusVar(w.config.color)}"></span>
          </span>
          <span class="name">{w.config.name}</span>
        </button>
        <button class="close" title="Close connection" aria-label="Close connection" on:click={() => dispatch("close", w.id)}>
          <svg viewBox="0 0 12 12" width="9" height="9" aria-hidden="true"><path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
        </button>
      </div>
    {/each}
  </div>

  <button class="add" title="New connection" aria-label="New connection" on:click={() => dispatch("add")}>
    <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true"><path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>
  </button>
</nav>

<style>
  .rail {
    width: 68px; flex: none; display: flex; flex-direction: column; align-items: center;
    padding: var(--s-3) 0; gap: var(--s-2);
    background: var(--bg-app); border-right: 1px solid var(--border);
    overflow-y: auto; overflow-x: hidden;
  }
  .rail::-webkit-scrollbar { width: 0; }

  .items { display: flex; flex-direction: column; align-items: stretch; gap: var(--s-1); width: 100%; }

  .slot { position: relative; width: 100%; }
  .ws {
    width: 100%; display: flex; flex-direction: column; align-items: center; gap: 3px;
    padding: var(--s-2) 2px; border-radius: var(--r-md); color: var(--muted);
    transition: background var(--t-fast) var(--ease-out), color var(--t-fast) var(--ease-out);
  }
  .ws:hover { background: var(--bg-panel); color: var(--ink-soft); }
  .slot.active .ws { background: var(--bg-panel); color: var(--ink); }

  .bar {
    position: absolute; left: 0; top: 50%; transform: translateY(-50%);
    width: 3px; height: 26px; border-radius: 0 2px 2px 0; background: var(--accent);
  }

  .badge {
    position: relative; width: 34px; height: 34px; border-radius: var(--r-md); flex: none;
    display: grid; place-items: center; color: #fff; font-weight: 700; font-size: 12px;
    background: var(--c); box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.22);
  }
  .slot.active .badge { box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.22), 0 0 0 2px color-mix(in srgb, var(--c) 60%, transparent); }
  .status {
    position: absolute; right: -2px; bottom: -2px; width: 10px; height: 10px; border-radius: 50%;
    background: var(--s); border: 2px solid var(--bg-app);
  }
  .slot.active .status { border-color: var(--bg-panel); }

  .name {
    max-width: 100%; font-size: 9.5px; line-height: 1.1; text-align: center;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis; padding: 0 2px;
  }

  .close {
    position: absolute; top: -2px; right: 6px; width: 16px; height: 16px; border-radius: 50%;
    display: grid; place-items: center; color: var(--muted);
    background: var(--bg-elevated); box-shadow: 0 0 0 1px var(--border);
    opacity: 0; transition: opacity var(--t-fast) var(--ease-out);
  }
  .slot:hover .close { opacity: 1; }
  .close:hover { color: var(--danger); }

  .add {
    width: 40px; height: 40px; flex: none; margin-top: var(--s-1); border-radius: var(--r-md);
    display: grid; place-items: center; color: var(--muted);
    border: 1px dashed var(--border-strong);
    transition: background var(--t-fast) var(--ease-out), color var(--t-fast) var(--ease-out), border-color var(--t-fast) var(--ease-out);
  }
  .add:hover { background: var(--bg-panel); color: var(--ink); border-color: var(--muted); }
</style>
