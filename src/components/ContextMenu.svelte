<script lang="ts">
  import { contextMenu, closeContextMenu, type CtxItem } from "../lib/stores/contextMenu";

  let menuEl: HTMLDivElement;
  // Clamp to the viewport so the menu never opens off-screen.
  $: pos = $contextMenu
    ? {
        left: Math.min($contextMenu.x, window.innerWidth - 220),
        top: Math.min($contextMenu.y, window.innerHeight - estHeight($contextMenu.items)),
      }
    : { left: 0, top: 0 };

  function estHeight(items: CtxItem[]): number {
    return items.reduce((h, i) => h + (i.separator ? 9 : 28), 12);
  }
  function run(item: CtxItem) {
    if (item.disabled || !item.action) return;
    const fn = item.action;
    closeContextMenu();
    fn();
  }
</script>

<svelte:window
  on:keydown={(e) => { if (e.key === "Escape") closeContextMenu(); }}
  on:resize={closeContextMenu}
/>

{#if $contextMenu}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="ctx-backdrop" on:click={closeContextMenu} on:contextmenu|preventDefault={closeContextMenu}></div>
  <div class="ctx-menu" bind:this={menuEl} style="left: {pos.left}px; top: {pos.top}px;" role="menu">
    {#each $contextMenu.items as item, i (i)}
      {#if item.separator}
        <div class="ctx-sep" role="separator"></div>
      {:else}
        <button class="ctx-item" class:danger={item.danger} disabled={item.disabled} role="menuitem" on:click={() => run(item)}>
          {item.label}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .ctx-backdrop { position: fixed; inset: 0; z-index: var(--z-dropdown); }
  .ctx-menu {
    position: fixed; z-index: var(--z-dropdown); min-width: 200px; max-width: 280px;
    background: var(--bg-elevated); border: 1px solid var(--border-strong);
    border-radius: var(--r-md); box-shadow: var(--shadow-pop);
    padding: var(--s-1); display: flex; flex-direction: column;
  }
  .ctx-item {
    text-align: left; padding: var(--s-2) var(--s-3); border-radius: var(--r-sm);
    font-size: 12.5px; color: var(--ink-soft); background: none; white-space: nowrap;
    overflow: hidden; text-overflow: ellipsis;
  }
  .ctx-item:hover:not(:disabled) { background: var(--accent); color: var(--accent-ink); }
  .ctx-item.danger { color: var(--danger); }
  .ctx-item.danger:hover:not(:disabled) { background: var(--danger); color: #fff; }
  .ctx-item:disabled { opacity: 0.4; }
  .ctx-sep { height: 1px; margin: var(--s-1) var(--s-2); background: var(--hairline); }
</style>
