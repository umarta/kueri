<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import Modal from "./Modal.svelte";
  import { DB_KINDS } from "../lib/dbKinds";
  import type { DbKind } from "../lib/types";

  const dispatch = createEventDispatcher<{ pick: DbKind; close: void }>();

  function pick(value: DbKind, implemented: boolean) {
    if (!implemented) return;
    dispatch("pick", value);
  }
</script>

<Modal title="Choose a database" width="540px" on:close={() => dispatch("close")}>
  <div class="grid">
    {#each DB_KINDS as k (k.value)}
      <button
        class="tile"
        class:soon={!k.implemented}
        on:click={() => pick(k.value, k.implemented)}
        title={k.implemented ? k.label : `${k.label} — coming soon`}
      >
        <span class="badge" style="--c: {k.color}">{k.abbr}</span>
        <span class="name">{k.label}</span>
        {#if !k.implemented}<span class="soon-tag">soon</span>{/if}
      </button>
    {/each}
  </div>
  <svelte:fragment slot="footer">
    <button class="btn" on:click={() => dispatch("close")}>Cancel</button>
  </svelte:fragment>
</Modal>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--s-3);
  }
  .tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s-3);
    padding: var(--s-6) var(--s-2) var(--s-5);
    border-radius: var(--r-md);
    border: 1px solid transparent;
    transition: background var(--t-fast) var(--ease-out),
      border-color var(--t-fast) var(--ease-out);
    position: relative;
  }
  .tile:hover { background: var(--bg-elevated); border-color: var(--border); }
  .tile:active { background: var(--bg-active); }
  .tile:focus-visible { outline: none; border-color: var(--accent); box-shadow: 0 0 0 3px var(--focus-ring); }

  .badge {
    width: 46px;
    height: 46px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-weight: 700;
    font-size: 16px;
    color: #fff;
    background: var(--c);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.22), 0 1px 2px rgba(0, 0, 0, 0.3);
  }
  .name {
    font-size: 12px;
    color: var(--ink-soft);
    text-align: center;
    line-height: 1.3;
  }
  .soon { opacity: 0.42; cursor: default; }
  .soon:hover { background: transparent; border-color: transparent; }
  .soon-tag {
    position: absolute;
    top: var(--s-2);
    right: var(--s-2);
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--faint);
    border: 1px solid var(--border);
    border-radius: var(--r-xs);
    padding: 1px 4px;
  }
</style>
