<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { activeConnection } from "../lib/stores/connection";
  import { dbKind, statusVar } from "../lib/dbKinds";

  export let sidebarOpen = true;
  export let logOpen = false;
  export let readOnly = false;
  export let inTxn = false;
  export let txnBusy = false;

  const dispatch = createEventDispatcher<{
    disconnect: void;
    refresh: void;
    toggleSidebar: void;
    toggleSettings: void;
    toggleLog: void;
    toggleReadOnly: void;
    begin: void;
    commit: void;
    rollback: void;
  }>();

  $: conn = $activeConnection;
  $: meta = conn ? dbKind(conn.kind) : null;
</script>

<header class="toolbar">
  <div class="left">
    <button
      class="tbtn"
      title="Edit settings"
      aria-label="Edit settings"
      on:click={() => dispatch("toggleSettings")}
    >
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" aria-hidden="true">
        <path d="M15 12C15 13.6569 13.6569 15 12 15C10.3431 15 9 13.6569 9 12C9 10.3431 10.3431 9 12 9C13.6569 9 15 10.3431 15 12Z" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>
        <path d="M12.9046 3.06005C12.6988 3 12.4659 3 12 3C11.5341 3 11.3012 3 11.0954 3.06005C10.7942 3.14794 10.5281 3.32808 10.3346 3.57511C10.2024 3.74388 10.1159 3.96016 9.94291 4.39272C9.69419 5.01452 9.00393 5.33471 8.36857 5.123L7.79779 4.93281C7.3929 4.79785 7.19045 4.73036 6.99196 4.7188C6.70039 4.70181 6.4102 4.77032 6.15701 4.9159C5.98465 5.01501 5.83376 5.16591 5.53197 5.4677C5.21122 5.78845 5.05084 5.94882 4.94896 6.13189C4.79927 6.40084 4.73595 6.70934 4.76759 7.01551C4.78912 7.2239 4.87335 7.43449 5.04182 7.85566C5.30565 8.51523 5.05184 9.26878 4.44272 9.63433L4.16521 9.80087C3.74031 10.0558 3.52786 10.1833 3.37354 10.3588C3.23698 10.5141 3.13401 10.696 3.07109 10.893C3 11.1156 3 11.3658 3 11.8663C3 12.4589 3 12.7551 3.09462 13.0088C3.17823 13.2329 3.31422 13.4337 3.49124 13.5946C3.69158 13.7766 3.96395 13.8856 4.50866 14.1035C5.06534 14.3261 5.35196 14.9441 5.16236 15.5129L4.94721 16.1584C4.79819 16.6054 4.72367 16.829 4.7169 17.0486C4.70875 17.3127 4.77049 17.5742 4.89587 17.8067C5.00015 18.0002 5.16678 18.1668 5.5 18.5C5.83323 18.8332 5.99985 18.9998 6.19325 19.1041C6.4258 19.2295 6.68733 19.2913 6.9514 19.2831C7.17102 19.2763 7.39456 19.2018 7.84164 19.0528L8.36862 18.8771C9.00393 18.6654 9.6942 18.9855 9.94291 19.6073C10.1159 20.0398 10.2024 20.2561 10.3346 20.4249C10.5281 20.6719 10.7942 20.8521 11.0954 20.94C11.3012 21 11.5341 21 12 21C12.4659 21 12.6988 21 12.9046 20.94C13.2058 20.8521 13.4719 20.6719 13.6654 20.4249C13.7976 20.2561 13.8841 20.0398 14.0571 19.6073C14.3058 18.9855 14.9961 18.6654 15.6313 18.8773L16.1579 19.0529C16.605 19.2019 16.8286 19.2764 17.0482 19.2832C17.3123 19.2913 17.5738 19.2296 17.8063 19.1042C17.9997 18.9999 18.1664 18.8333 18.4996 18.5001C18.8328 18.1669 18.9994 18.0002 19.1037 17.8068C19.2291 17.5743 19.2908 17.3127 19.2827 17.0487C19.2759 16.8291 19.2014 16.6055 19.0524 16.1584L18.8374 15.5134C18.6477 14.9444 18.9344 14.3262 19.4913 14.1035C20.036 13.8856 20.3084 13.7766 20.5088 13.5946C20.6858 13.4337 20.8218 13.2329 20.9054 13.0088C21 12.7551 21 12.4589 21 11.8663C21 11.3658 21 11.1156 20.9289 10.893C20.866 10.696 20.763 10.5141 20.6265 10.3588C20.4721 10.1833 20.2597 10.0558 19.8348 9.80087L19.5569 9.63416C18.9478 9.26867 18.6939 8.51514 18.9578 7.85558C19.1262 7.43443 19.2105 7.22383 19.232 7.01543C19.2636 6.70926 19.2003 6.40077 19.0506 6.13181C18.9487 5.94875 18.7884 5.78837 18.4676 5.46762C18.1658 5.16584 18.0149 5.01494 17.8426 4.91583C17.5894 4.77024 17.2992 4.70174 17.0076 4.71872C16.8091 4.73029 16.6067 4.79777 16.2018 4.93273L15.6314 5.12287C14.9961 5.33464 14.3058 5.0145 14.0571 4.39272C13.8841 3.96016 13.7976 3.74388 13.6654 3.57511C13.4719 3.32808 13.2058 3.14794 12.9046 3.06005Z" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>
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
    {#if conn}
      {#if inTxn}
        <span class="txn-badge" title="A transaction is open — changes are uncommitted">TXN</span>
        <button class="ttext commit" disabled={txnBusy} on:click={() => dispatch("commit")} title="Commit the transaction">Commit</button>
        <button class="ttext rollback" disabled={txnBusy} on:click={() => dispatch("rollback")} title="Roll back the transaction">Rollback</button>
      {:else}
        <button class="ttext" disabled={txnBusy} on:click={() => dispatch("begin")} title="Begin a transaction (manual commit)">Begin</button>
      {/if}
      <span class="divider"></span>
    {/if}
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

  .ttext { height: 24px; padding: 0 var(--s-3); border-radius: var(--r-sm); font-size: 11.5px; font-weight: 600; color: var(--ink-soft); }
  .ttext:hover:not(:disabled) { background: var(--bg-elevated); color: var(--ink); }
  .ttext:disabled { opacity: 0.5; }
  .ttext.commit { color: var(--ok, #18a558); }
  .ttext.commit:hover:not(:disabled) { background: color-mix(in srgb, var(--ok, #18a558) 14%, transparent); }
  .ttext.rollback { color: var(--danger); }
  .ttext.rollback:hover:not(:disabled) { background: var(--danger-soft); }
  .txn-badge { align-self: center; padding: 1px var(--s-2); border-radius: var(--r-xs); background: var(--warn); color: #1a1206; font-size: 9.5px; font-weight: 800; letter-spacing: 0.05em; }

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
