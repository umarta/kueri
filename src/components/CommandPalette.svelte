<script lang="ts">
  import { onMount, createEventDispatcher } from "svelte";
  import { api } from "../lib/tauri";
  import { activeConnectionId } from "../lib/stores/connection";

  const dispatch = createEventDispatcher<{ select: { schema: string; table: string }; close: void }>();

  let q = "";
  let items: { schema: string; table: string }[] = [];
  let active = 0;
  let loading = true;
  let inputEl: HTMLInputElement;

  onMount(async () => {
    inputEl?.focus();
    const id = $activeConnectionId;
    if (!id) {
      loading = false;
      return;
    }
    try {
      const schemas = await api.listSchemas(id);
      const groups = await Promise.all(
        schemas.map((s) =>
          api.listTables(id, s.name).then((ts) => ts.map((t) => ({ schema: s.name, table: t.name }))),
        ),
      );
      items = groups.flat();
    } finally {
      loading = false;
    }
  });

  $: filtered = (
    q ? items.filter((i) => `${i.table} ${i.schema}`.toLowerCase().includes(q.toLowerCase())) : items
  ).slice(0, 80);
  $: if (active >= filtered.length) active = Math.max(0, filtered.length - 1);

  function choose(it: { schema: string; table: string } | undefined) {
    if (it) dispatch("select", it);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "ArrowDown") { e.preventDefault(); active = Math.min(active + 1, filtered.length - 1); }
    else if (e.key === "ArrowUp") { e.preventDefault(); active = Math.max(active - 1, 0); }
    else if (e.key === "Enter") { e.preventDefault(); choose(filtered[active]); }
    else if (e.key === "Escape") { e.preventDefault(); dispatch("close"); }
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
<div class="pal-backdrop" on:click|self={() => dispatch("close")}>
  <div class="pal" role="dialog" aria-label="Open table">
    <div class="pal-search">
      <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><circle cx="7" cy="7" r="4.5" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M11 11l3 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
      <input bind:this={inputEl} bind:value={q} on:keydown={onKey} placeholder="Search tables…" spellcheck="false" />
      <span class="kbd">esc</span>
    </div>
    <div class="pal-list">
      {#if loading}
        <div class="pal-empty">Loading…</div>
      {:else if filtered.length === 0}
        <div class="pal-empty">No matches</div>
      {:else}
        {#each filtered as it, i (it.schema + "." + it.table)}
          <button class="pal-item" class:active={i === active} on:click={() => choose(it)} on:mousemove={() => (active = i)}>
            <svg class="pi-icon" viewBox="0 0 16 16" width="13" height="13" aria-hidden="true">
              <rect x="2.5" y="3" width="11" height="10" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.2"/>
              <line x1="2.5" y1="6.3" x2="13.5" y2="6.3" stroke="currentColor" stroke-width="1.2"/>
              <line x1="6.5" y1="6.3" x2="6.5" y2="13" stroke="currentColor" stroke-width="1.2"/>
            </svg>
            <span class="pi-name">{it.table}</span>
            <span class="pi-schema">{it.schema}</span>
          </button>
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
  .pal-backdrop { position: fixed; inset: 0; z-index: var(--z-modal); display: flex; justify-content: center; align-items: flex-start; padding-top: 12vh; background: rgba(0, 0, 0, 0.4); }
  .pal { width: min(560px, 92vw); max-height: 60vh; display: flex; flex-direction: column; background: var(--bg-panel); border: 1px solid var(--border-strong); border-radius: var(--r-lg); box-shadow: var(--shadow-modal); overflow: hidden; }
  .pal-search { display: flex; align-items: center; gap: var(--s-3); padding: var(--s-4) var(--s-5); border-bottom: 1px solid var(--hairline); color: var(--faint); }
  .pal-search input { flex: 1; min-width: 0; background: none; border: none; outline: none; color: var(--ink); font: inherit; font-size: 14px; }
  .pal-search input::placeholder { color: var(--faint); }
  .kbd { font-size: 10px; color: var(--faint); border: 1px solid var(--border); border-radius: var(--r-xs); padding: 1px 5px; }

  .pal-list { flex: 1; overflow-y: auto; padding: var(--s-2); }
  .pal-item { width: 100%; display: flex; align-items: center; gap: var(--s-3); padding: var(--s-2) var(--s-3); border-radius: var(--r-sm); text-align: left; color: var(--ink-soft); }
  .pal-item.active { background: var(--accent); color: var(--accent-ink); }
  .pal-item.active .pi-icon, .pal-item.active .pi-schema { color: var(--accent-ink); opacity: 0.9; }
  .pi-icon { color: var(--muted); flex: none; }
  .pi-name { font-size: 13px; font-family: var(--font-mono); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pi-schema { margin-left: auto; font-size: 11px; color: var(--faint); flex: none; }
  .pal-empty { padding: var(--s-6); text-align: center; color: var(--faint); font-size: 12.5px; }
</style>
