<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { activeConnection } from "../lib/stores/connection";
  import { dbKind, statusVar } from "../lib/dbKinds";

  export let sidebarOpen = true;
  export let logOpen = false;
  export let detailOpen = false;
  export let readOnly = false;

  const dispatch = createEventDispatcher<{
    disconnect: void;
    refresh: void;
    toggleSidebar: void;
    toggleLog: void;
    toggleDetail: void;
    toggleReadOnly: void;
  }>();

  $: conn = $activeConnection;
  $: meta = conn ? dbKind(conn.kind) : null;
</script>

<header class="toolbar">
  <div class="left">
    <button
      class="tbtn"
      class:active={sidebarOpen}
      title="Toggle sidebar"
      aria-label="Toggle sidebar"
      on:click={() => dispatch("toggleSidebar")}
    >
      <svg viewBox="0 0 18 18" width="16" height="16" aria-hidden="true">
        <rect x="2" y="3" width="14" height="12" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.4"/>
        <line x1="7" y1="3" x2="7" y2="15" stroke="currentColor" stroke-width="1.4"/>
      </svg>
    </button>
  </div>

  <div class="center">
    {#if conn && meta}
      <div class="ident">
        <span class="dot" style="--c: {statusVar(conn.color)}"></span>
        <span class="badge" style="--c: {meta.color}">{meta.abbr}</span>
        {#if conn.tag}<span class="env">{conn.tag.toUpperCase()}</span><span class="sep">·</span>{/if}
        <span class="name">{conn.name}</span>
        <span class="sep">·</span>
        <span class="db">{conn.kind === "sqlite" ? conn.file_path ?? conn.database : conn.database || meta.label}</span>
      </div>
    {/if}
  </div>

  <div class="right">
    <button
      class="tbtn lock"
      class:locked={readOnly}
      title={readOnly ? "Read-only mode on — click to allow writes" : "Writes allowed — click for read-only mode"}
      aria-label="Toggle read-only mode"
      on:click={() => dispatch("toggleReadOnly")}
    >
      {#if readOnly}
        <svg viewBox="0 0 18 18" width="15" height="15" aria-hidden="true"><rect x="4" y="8" width="10" height="7" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.4"/><path d="M6 8V6a3 3 0 0 1 6 0v2" fill="none" stroke="currentColor" stroke-width="1.4"/></svg>
      {:else}
        <svg viewBox="0 0 18 18" width="15" height="15" aria-hidden="true"><rect x="4" y="8" width="10" height="7" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.4"/><path d="M6 8V6a3 3 0 0 1 5.8-1" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg>
      {/if}
    </button>
    <span class="divider"></span>
    <button
      class="tbtn"
      class:active={logOpen}
      title="Toggle query log"
      aria-label="Toggle query log"
      on:click={() => dispatch("toggleLog")}
    >
      <svg viewBox="0 0 18 18" width="16" height="16" aria-hidden="true">
        <rect x="2" y="3" width="14" height="12" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.4"/>
        <line x1="2" y1="11.5" x2="16" y2="11.5" stroke="currentColor" stroke-width="1.4"/>
      </svg>
    </button>
    <button
      class="tbtn"
      class:active={detailOpen}
      title="Toggle row detail"
      aria-label="Toggle row detail"
      on:click={() => dispatch("toggleDetail")}
    >
      <svg viewBox="0 0 18 18" width="16" height="16" aria-hidden="true">
        <rect x="2" y="3" width="14" height="12" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.4"/>
        <line x1="11.5" y1="3" x2="11.5" y2="15" stroke="currentColor" stroke-width="1.4"/>
      </svg>
    </button>
    <span class="divider"></span>
    <button class="tbtn" title="Refresh" aria-label="Refresh" on:click={() => dispatch("refresh")}>
      <svg viewBox="0 0 18 18" width="15" height="15" aria-hidden="true"><path d="M14.5 9a5.5 5.5 0 1 1-1.6-3.9M14.5 3v3h-3" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
    </button>
    <button class="tbtn" title="Close connection" aria-label="Close connection" on:click={() => dispatch("disconnect")}>
      <svg viewBox="0 0 18 18" width="15" height="15" aria-hidden="true"><path d="M11 3h3.5v3.5M14 3l-5.5 5.5M8 4H4.5A1.5 1.5 0 0 0 3 5.5v8A1.5 1.5 0 0 0 4.5 15h8a1.5 1.5 0 0 0 1.5-1.5V10" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>
    </button>
  </div>
</header>

<style>
  .toolbar {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    height: 44px;
    padding: 0 var(--s-4);
    background: var(--bg-panel);
    border-bottom: 1px solid var(--border);
    -webkit-app-region: drag; /* native title-bar feel if used as overlay */
  }
  .left { display: flex; gap: var(--s-1); padding-left: 72px; } /* clear macOS traffic lights */
  .right { display: flex; gap: var(--s-1); justify-content: flex-end; }
  .left, .right, .center { -webkit-app-region: no-drag; }

  .tbtn {
    width: 28px; height: 28px; border-radius: var(--r-sm);
    display: grid; place-items: center; color: var(--muted);
    transition: background var(--t-fast) var(--ease-out), color var(--t-fast) var(--ease-out);
  }
  .tbtn:hover { background: var(--bg-elevated); color: var(--ink); }
  .tbtn.active { background: var(--bg-active, var(--bg-elevated)); color: var(--accent); }
  .tbtn.lock.locked { color: var(--warn); }
  .divider { width: 1px; height: 18px; background: var(--border); margin: 0 var(--s-1); align-self: center; }

  .center { display: flex; justify-content: center; }
  .ident {
    display: flex; align-items: center; gap: var(--s-3);
    height: 28px; padding: 0 var(--s-5);
    background: var(--bg-content); border: 1px solid var(--border);
    border-radius: var(--r-sm); max-width: 60vw;
    font-size: 12px; color: var(--ink-soft);
  }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--c); flex: none; }
  .badge {
    width: 18px; height: 18px; border-radius: 50%; flex: none;
    display: grid; place-items: center; color: #fff; font-weight: 700; font-size: 9px;
    background: var(--c);
  }
  .env { font-weight: 700; font-size: 10.5px; letter-spacing: 0.04em; color: var(--muted); }
  .name { font-weight: 600; color: var(--ink); white-space: nowrap; }
  .db { font-family: var(--font-mono); color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sep { color: var(--faint); }
</style>
