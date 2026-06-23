<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { settings } from "../lib/stores/settings";

  const dispatch = createEventDispatcher<{ close: void }>();

  let section: "general" | "shortcuts" | "about" = "general";

  const shortcuts: { keys: string; label: string }[] = [
    { keys: "⌘P", label: "Open anything (search a table)" },
    { keys: "⌘T", label: "New query tab" },
    { keys: "⌘E", label: "New SQL editor" },
    { keys: "⌘W", label: "Close tab" },
    { keys: "⌘[ / ⌘]", label: "Previous / next tab" },
    { keys: "⌘1–9", label: "Jump to tab" },
    { keys: "⌘N", label: "New connection" },
    { keys: "⌘R", label: "Reload" },
    { keys: "⌘K", label: "Switch schema" },
    { keys: "⌘S", label: "Commit changes" },
    { keys: "⌘I", label: "Add row" },
    { keys: "⌘F", label: "Toggle filters" },
    { keys: "Space", label: "Toggle row detail" },
    { keys: "⌘⌃[ / ⌘⌃]", label: "Data / Structure view" },
    { keys: "⌘↵", label: "Run query" },
    { keys: "⌘,", label: "Settings" },
  ];

  function setLimit(v: string) {
    const n = parseInt(v, 10);
    if (Number.isFinite(n) && n > 0) settings.update((s) => ({ ...s, rowLimit: Math.min(n, 100000) }));
  }

  function setTheme(v: string) {
    const theme = v === "light" || v === "dark" ? v : "auto";
    settings.update((s) => ({ ...s, theme }));
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
<div class="backdrop" on:click|self={() => dispatch("close")}>
  <div class="panel" role="dialog" aria-label="Settings">
    <header class="tabs">
      <button class:active={section === "general"} on:click={() => (section = "general")}>General</button>
      <button class:active={section === "shortcuts"} on:click={() => (section = "shortcuts")}>Shortcuts</button>
      <button class:active={section === "about"} on:click={() => (section = "about")}>About</button>
      <div class="sp"></div>
      <button class="x" on:click={() => dispatch("close")} aria-label="Close" title="Close (Esc)">
        <svg viewBox="0 0 14 14" width="13" height="13" aria-hidden="true"><path d="M3.5 3.5l7 7M10.5 3.5l-7 7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg>
      </button>
    </header>

    <div class="body">
      {#if section === "general"}
        <div class="row">
          <div class="lbl"><span class="name">Theme</span><span class="desc">Auto follows your operating system.</span></div>
          <select class="num" value={$settings.theme} on:change={(e) => setTheme(e.currentTarget.value)}>
            <option value="auto">Auto</option>
            <option value="light">Light</option>
            <option value="dark">Dark</option>
          </select>
        </div>
        <div class="row">
          <div class="lbl"><span class="name">Client tools folder</span><span class="desc">Folder with pg_dump/mysqldump etc. Set this if your PATH copy is older than the server.</span></div>
          <input class="num wide" value={$settings.toolsPath} placeholder="/opt/homebrew/opt/postgresql@17/bin" on:change={(e) => settings.update((s) => ({ ...s, toolsPath: e.currentTarget.value.trim() }))} />
        </div>
        <div class="row">
          <div class="lbl"><span class="name">Default row limit</span><span class="desc">Rows fetched when browsing a table.</span></div>
          <input class="num" type="number" min="1" value={$settings.rowLimit} on:change={(e) => setLimit(e.currentTarget.value)} />
        </div>
        <div class="row">
          <div class="lbl"><span class="name">Alternating row colors</span><span class="desc">Tint every other row in the grid.</span></div>
          <label class="switch"><input type="checkbox" checked={$settings.altRows} on:change={(e) => settings.update((s) => ({ ...s, altRows: e.currentTarget.checked }))} /></label>
        </div>
      {:else if section === "shortcuts"}
        <ul class="keys">
          {#each shortcuts as s (s.label)}
            <li><span class="combo">{s.keys}</span><span class="act">{s.label}</span></li>
          {/each}
        </ul>
      {:else}
        <div class="about">
          <div class="logo">Kueri</div>
          <p class="ver">Version 0.1.0</p>
          <p class="tag">A lightweight, native, open-source multi-database client.</p>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; z-index: var(--z-modal); display: grid; place-items: center; background: rgba(0, 0, 0, 0.45); }
  .panel { width: min(560px, 92vw); max-height: 78vh; display: flex; flex-direction: column; background: var(--bg-panel); border: 1px solid var(--border-strong); border-radius: var(--r-lg); box-shadow: var(--shadow-modal); overflow: hidden; }

  .tabs { display: flex; align-items: center; gap: var(--s-1); padding: var(--s-2) var(--s-3); border-bottom: 1px solid var(--hairline); flex: none; }
  .tabs button { padding: var(--s-2) var(--s-3); border-radius: var(--r-sm); font-size: 12.5px; font-weight: 500; color: var(--muted); }
  .tabs button:hover { color: var(--ink); }
  .tabs button.active { background: var(--bg-elevated); color: var(--ink); }
  .tabs .sp { flex: 1; }
  .tabs .x { width: 26px; height: 26px; display: grid; place-items: center; color: var(--muted); }
  .tabs .x:hover { background: var(--bg-elevated); color: var(--ink); }

  .body { padding: var(--s-5) var(--s-6); overflow-y: auto; }

  .row { display: flex; align-items: center; justify-content: space-between; gap: var(--s-5); padding: var(--s-4) 0; border-bottom: 1px solid var(--hairline); }
  .row:last-child { border-bottom: none; }
  .lbl { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .name { font-size: 13px; color: var(--ink); }
  .desc { font-size: 11.5px; color: var(--muted); }
  .num { width: 96px; height: 30px; padding: 0 var(--s-3); background: var(--bg-content); border: 1px solid var(--border); border-radius: var(--r-sm); color: var(--ink); font: inherit; font-size: 13px; text-align: right; }
  .num:focus { outline: none; border-color: var(--accent); }
  .num.wide { width: 260px; text-align: left; font-family: var(--font-mono); font-size: 12px; }
  .switch input { width: 16px; height: 16px; accent-color: var(--accent); }

  .keys { list-style: none; margin: 0; padding: 0; }
  .keys li { display: flex; align-items: center; gap: var(--s-5); padding: var(--s-2) 0; }
  .combo { width: 110px; flex: none; font-size: 12px; font-family: var(--font-mono); color: var(--ink); }
  .act { font-size: 12.5px; color: var(--muted); }

  .about { text-align: center; padding: var(--s-7) var(--s-4); }
  .about .logo { font-size: 22px; font-weight: 700; letter-spacing: -0.02em; color: var(--ink); }
  .about .ver { margin: var(--s-2) 0 0; font-size: 12px; color: var(--faint); }
  .about .tag { margin: var(--s-4) 0 0; font-size: 12.5px; color: var(--muted); }
</style>
