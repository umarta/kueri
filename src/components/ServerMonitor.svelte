<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import { api } from "../lib/tauri";
  import type { ProcessInfo, RoleInfo } from "../lib/types";

  export let connectionId: string;

  const dispatch = createEventDispatcher<{ close: void }>();

  let view: "processes" | "users" = "processes";
  let processes: ProcessInfo[] = [];
  let roles: RoleInfo[] = [];
  let busy = false;
  let error = "";

  onMount(refresh);

  async function refresh() {
    busy = true;
    error = "";
    try {
      processes = await api.listProcesses(connectionId);
      roles = await api.listRoles(connectionId);
    } catch (e) {
      error = (e as { message?: string })?.message ?? String(e);
    } finally {
      busy = false;
    }
  }
  async function kill(pid: string) {
    try {
      await api.killProcess(connectionId, pid);
      await refresh();
    } catch (e) {
      error = (e as { message?: string })?.message ?? String(e);
    }
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
<div class="backdrop" on:click|self={() => dispatch("close")}>
  <div class="dialog" role="dialog" aria-modal="true">
    <header class="dhead">
      <div class="vtabs">
        <button class:on={view === "processes"} on:click={() => (view = "processes")}>Processes <span class="n">{processes.length}</span></button>
        <button class:on={view === "users"} on:click={() => (view = "users")}>Users <span class="n">{roles.length}</span></button>
      </div>
      <div class="spacer"></div>
      <button class="hbtn" on:click={refresh} disabled={busy}>{busy ? "…" : "Refresh"}</button>
      <button class="x" on:click={() => dispatch("close")} aria-label="Close">✕</button>
    </header>

    {#if error}<div class="err">{error}</div>{/if}

    <div class="body">
      {#if view === "processes"}
        {#if processes.length === 0}
          <p class="empty">{busy ? "Loading…" : "No active queries."}</p>
        {:else}
          <table class="grid">
            <thead><tr><th>pid</th><th>user</th><th>db</th><th>state</th><th>secs</th><th>query</th><th></th></tr></thead>
            <tbody>
              {#each processes as p (p.pid)}
                <tr>
                  <td class="mono">{p.pid}</td>
                  <td>{p.user}</td>
                  <td>{p.database}</td>
                  <td>{p.state}</td>
                  <td class="mono num">{p.seconds}</td>
                  <td class="q" title={p.query}>{p.query}</td>
                  <td><button class="kill" on:click={() => kill(p.pid)} title="Terminate">Kill</button></td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      {:else}
        {#if roles.length === 0}
          <p class="empty">{busy ? "Loading…" : "No roles found."}</p>
        {:else}
          <table class="grid">
            <thead><tr><th>name</th><th>attributes</th></tr></thead>
            <tbody>
              {#each roles as r (r.name + r.attributes)}
                <tr><td class="mono">{r.name}</td><td class="dim">{r.attributes}</td></tr>
              {/each}
            </tbody>
          </table>
        {/if}
      {/if}
    </div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; z-index: var(--z-modal); display: grid; place-items: start center; padding: 8vh 40px; background: rgba(0,0,0,0.45); }
  .dialog { width: min(820px, 96vw); max-height: 78vh; display: flex; flex-direction: column; background: var(--bg-panel); border: 1px solid var(--border-strong); border-radius: var(--r-lg); box-shadow: var(--shadow-modal); overflow: hidden; }
  .dhead { display: flex; align-items: center; gap: var(--s-2); padding: var(--s-2) var(--s-3); border-bottom: 1px solid var(--hairline); flex: none; }
  .vtabs { display: flex; gap: 2px; }
  .vtabs button { padding: var(--s-2) var(--s-3); border-radius: var(--r-sm); font-size: 12.5px; font-weight: 600; color: var(--muted); }
  .vtabs button.on { color: var(--accent); background: var(--bg-elevated); }
  .vtabs .n { font-size: 10px; color: var(--faint); }
  .spacer { flex: 1; }
  .hbtn { font-size: 12px; color: var(--ink-soft); padding: var(--s-1) var(--s-3); border-radius: var(--r-sm); }
  .hbtn:hover:not(:disabled) { background: var(--bg-elevated); }
  .x { width: 24px; height: 24px; border-radius: var(--r-sm); color: var(--muted); }
  .x:hover { background: var(--bg-elevated); color: var(--ink); }
  .err { padding: var(--s-2) var(--s-3); background: var(--danger-soft); color: var(--danger); font-size: 12px; white-space: pre-wrap; }
  .empty { padding: var(--s-7); text-align: center; color: var(--faint); font-size: 12.5px; }

  .body { overflow: auto; }
  .grid { border-collapse: separate; border-spacing: 0; width: 100%; }
  .grid th, .grid td { text-align: left; padding: var(--s-2) var(--s-3); border-bottom: 1px solid var(--hairline); font-size: 12px; white-space: nowrap; }
  .grid th { position: sticky; top: 0; background: var(--bg-panel); color: var(--muted); font-family: var(--font-mono); font-weight: 600; font-size: 11px; }
  .grid tbody tr:hover td { background: var(--bg-elevated); }
  .mono { font-family: var(--font-mono); }
  .num { text-align: right; }
  .dim { color: var(--muted); }
  .q { font-family: var(--font-mono); color: var(--ink-soft); max-width: 360px; overflow: hidden; text-overflow: ellipsis; }
  .kill { font-size: 11px; color: var(--danger); padding: 1px var(--s-2); border-radius: var(--r-xs); }
  .kill:hover { background: var(--danger-soft); }
</style>
