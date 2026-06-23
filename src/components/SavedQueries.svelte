<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import { savedQueries, addSaved, removeSaved } from "../lib/stores/saved";

  /** SQL of the active query tab (to save), if any. */
  export let currentSql = "";

  const dispatch = createEventDispatcher<{ open: string; close: void }>();

  let name = "";
  let q = "";
  let nameInput: HTMLInputElement;

  $: shown = q.trim()
    ? $savedQueries.filter(
        (s) => s.name.toLowerCase().includes(q.trim().toLowerCase()) || s.sql.toLowerCase().includes(q.trim().toLowerCase()),
      )
    : $savedQueries;

  onMount(() => nameInput?.focus());

  function save() {
    if (!currentSql.trim()) return;
    addSaved(name, currentSql.trim());
    name = "";
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
<div class="backdrop" on:click|self={() => dispatch("close")}>
  <div class="dialog" role="dialog" aria-modal="true">
    <header class="dhead">
      <span class="dtitle">Saved queries</span>
      <button class="x" on:click={() => dispatch("close")} aria-label="Close">✕</button>
    </header>

    {#if currentSql.trim()}
      <div class="saverow">
        <input bind:this={nameInput} bind:value={name} placeholder="Name this query…" spellcheck="false"
          on:keydown={(e) => { if (e.key === "Enter") save(); }} />
        <button class="btn primary" on:click={save} disabled={!name.trim()}>Save current</button>
      </div>
    {/if}

    <div class="searchrow">
      <input bind:value={q} placeholder="Search saved queries…" spellcheck="false" />
    </div>

    <div class="list">
      {#if shown.length === 0}
        <p class="empty">{$savedQueries.length === 0 ? "No saved queries yet." : "No matches."}</p>
      {:else}
        {#each shown as s (s.id)}
          <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
          <div class="srow" on:click={() => dispatch("open", s.sql)} title="Open in a new query tab">
            <div class="sinfo">
              <span class="sname">{s.name}</span>
              <code class="ssql">{s.sql}</code>
            </div>
            <button class="del" on:click|stopPropagation={() => removeSaved(s.id)} aria-label="Delete" title="Delete">✕</button>
          </div>
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; z-index: var(--z-modal); display: grid; place-items: start center; padding: 10vh 40px; background: rgba(0,0,0,0.45); }
  .dialog { width: min(640px, 100%); max-height: 70vh; display: flex; flex-direction: column; background: var(--bg-panel); border: 1px solid var(--border-strong); border-radius: var(--r-lg); box-shadow: var(--shadow-modal); overflow: hidden; }
  .dhead { display: flex; align-items: center; justify-content: space-between; padding: var(--s-3) var(--s-4); border-bottom: 1px solid var(--hairline); flex: none; }
  .dtitle { font-size: 13px; font-weight: 600; color: var(--ink); }
  .x { width: 24px; height: 24px; border-radius: var(--r-sm); color: var(--muted); }
  .x:hover { background: var(--bg-elevated); color: var(--ink); }

  .saverow, .searchrow { display: flex; gap: var(--s-2); padding: var(--s-3) var(--s-4); border-bottom: 1px solid var(--hairline); flex: none; }
  .saverow input, .searchrow input { flex: 1; min-width: 0; height: 30px; background: var(--bg-content); border: 1px solid var(--border); border-radius: var(--r-sm); color: var(--ink); font: inherit; font-size: 12.5px; padding: 0 var(--s-3); }
  .saverow input:focus, .searchrow input:focus { outline: none; border-color: var(--accent); }
  .btn { height: 30px; padding: 0 var(--s-4); border-radius: var(--r-sm); font: inherit; font-size: 12.5px; font-weight: 600; border: 1px solid transparent; }
  .btn.primary { background: var(--accent); color: var(--accent-ink); }
  .btn.primary:disabled { opacity: 0.5; }

  .list { overflow-y: auto; padding: var(--s-1); }
  .empty { padding: var(--s-6); text-align: center; color: var(--faint); font-size: 12.5px; }
  .srow { display: flex; align-items: center; gap: var(--s-3); padding: var(--s-2) var(--s-3); border-radius: var(--r-sm); cursor: pointer; }
  .srow:hover { background: var(--bg-elevated); }
  .sinfo { min-width: 0; flex: 1; }
  .sname { display: block; font-size: 12.5px; font-weight: 600; color: var(--ink); }
  .ssql { display: block; font-family: var(--font-mono); font-size: 11px; color: var(--faint); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .del { width: 22px; height: 22px; border-radius: var(--r-xs); color: var(--faint); flex: none; }
  .del:hover { background: var(--danger-soft); color: var(--danger); }
</style>
