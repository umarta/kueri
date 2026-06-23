<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import DbPicker from "./DbPicker.svelte";
  import ConnectionForm from "./ConnectionForm.svelte";
  import { api } from "../lib/tauri";
  import { savedConnections, removeConnection, resolvePassword } from "../lib/stores/connection";
  import { dbKind, statusVar } from "../lib/dbKinds";
  import type { ConnectionConfig, DbKind } from "../lib/types";

  const dispatch = createEventDispatcher<{ connected: { id: string; config: ConnectionConfig }; cancel: void }>();

  /** When opened as the "+ new connection" overlay (other connections already open). */
  export let dismissable = false;

  let query = "";
  let picking = false;
  let editing: ConnectionConfig | null = null;

  let connectingId: string | null = null;
  let error: string | null = null;

  $: list = $savedConnections.filter((c) =>
    `${c.name} ${c.tag ?? ""} ${c.group ?? ""} ${c.host} ${c.database}`.toLowerCase().includes(query.toLowerCase())
  );

  // Group the list into folders (ungrouped first, then named groups; collapsible).
  type Row = { type: "header"; name: string; count: number; open: boolean } | { type: "conn"; c: ConnectionConfig };
  let collapsed = new Set<string>(loadCollapsed());
  function loadCollapsed(): string[] {
    try { return JSON.parse(localStorage.getItem("kueri.conngroups") || "[]"); } catch { return []; }
  }
  function toggleGroup(g: string) {
    const s = new Set(collapsed);
    if (s.has(g)) s.delete(g); else s.add(g);
    collapsed = s;
    try { localStorage.setItem("kueri.conngroups", JSON.stringify([...collapsed])); } catch { /* ignore */ }
  }
  $: rows = buildRows(list, collapsed);
  function buildRows(items: ConnectionConfig[], col: Set<string>): Row[] {
    const groups = new Map<string, ConnectionConfig[]>();
    for (const c of items) {
      const g = (c.group ?? "").trim();
      if (!groups.has(g)) groups.set(g, []);
      groups.get(g)!.push(c);
    }
    const out: Row[] = [];
    for (const c of groups.get("") ?? []) out.push({ type: "conn", c });
    groups.delete("");
    for (const g of [...groups.keys()].sort((a, b) => a.localeCompare(b))) {
      const open = !col.has(g);
      out.push({ type: "header", name: g, count: groups.get(g)!.length, open });
      if (open) for (const c of groups.get(g)!) out.push({ type: "conn", c });
    }
    return out;
  }

  function subtitle(c: ConnectionConfig): string {
    if (c.kind === "sqlite") return c.file_path || c.database || "—";
    return `${c.host}${c.port ? ":" + c.port : ""}${c.database ? " · " + c.database : ""}`;
  }

  function newConnection() {
    picking = true;
  }

  function onPick(e: CustomEvent<DbKind>) {
    const meta = dbKind(e.detail);
    picking = false;
    editing = {
      id: crypto.randomUUID(),
      name: meta.label,
      kind: e.detail,
      host: "localhost",
      port: meta.port,
      database: "",
      user: "",
      password: "",
      ssl: false,
      file_path: null,
      color: "local",
      tag: "local",
    };
  }

  function edit(c: ConnectionConfig, e: MouseEvent) {
    e.stopPropagation();
    editing = { ...c };
  }

  function del(c: ConnectionConfig, e: MouseEvent) {
    e.stopPropagation();
    removeConnection(c.id);
  }

  async function open(c: ConnectionConfig) {
    if (connectingId) return;
    connectingId = c.id;
    error = null;
    try {
      // Pull the password from the keychain (saved connections don't keep it in memory).
      const cfg = { ...c, password: await resolvePassword(c) };
      const id = await api.connect(cfg);
      dispatch("connected", { id, config: cfg });
    } catch (e) {
      error = String(e);
    } finally {
      connectingId = null;
    }
  }
</script>

<svelte:window on:keydown={(e) => { if (dismissable && e.key === "Escape") dispatch("cancel"); }} />

<div class="welcome">
  {#if dismissable}
    <button class="dismiss" on:click={() => dispatch("cancel")} title="Cancel (Esc)" aria-label="Cancel">
      <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
    </button>
  {/if}
  <aside class="brand">
    <div class="logo">
      <svg viewBox="0 0 64 64" width="60" height="60" aria-hidden="true">
        <defs>
          <linearGradient id="kueri-db" x1="0" y1="0" x2="0.7" y2="1">
            <stop offset="0" stop-color="#5cc0ff" />
            <stop offset="1" stop-color="#0a84ff" />
          </linearGradient>
          <linearGradient id="kueri-lens" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stop-color="#73c8ff" />
            <stop offset="1" stop-color="#1488ff" />
          </linearGradient>
        </defs>
        <path d="M8 13 C8 9.5 38 9.5 38 13 L38 35 C38 38.5 8 38.5 8 35 Z" fill="url(#kueri-db)" />
        <ellipse cx="23" cy="13" rx="15" ry="4.6" fill="#7fcdff" />
        <path d="M8 21.5 C8 25 38 25 38 21.5" fill="none" stroke="#ffffff" stroke-opacity="0.45" stroke-width="1.4" />
        <path d="M8 28.5 C8 32 38 32 38 28.5" fill="none" stroke="#ffffff" stroke-opacity="0.45" stroke-width="1.4" />
        <line x1="49" y1="47" x2="57" y2="55" stroke="url(#kueri-db)" stroke-width="6" stroke-linecap="round" />
        <line x1="49" y1="47" x2="57" y2="55" stroke="#0a84ff" stroke-width="2.4" stroke-linecap="round" />
        <circle cx="42" cy="40" r="13" fill="url(#kueri-lens)" stroke="#eaf6ff" stroke-width="2.4" />
        <g fill="#ffffff">
          <rect x="36" y="40" width="3.2" height="3.2" rx="0.6" />
          <rect x="41.5" y="36.5" width="4" height="4" rx="0.7" />
          <rect x="45.5" y="41.5" width="3" height="3" rx="0.6" />
          <rect x="39.5" y="44.5" width="2.4" height="2.4" rx="0.5" />
          <circle cx="47.5" cy="38" r="1.1" />
        </g>
      </svg>
    </div>
    <h1>Kueri</h1>
    <p class="version">Version 0.1.0</p>

    <div class="actions">
      <button class="action" on:click={newConnection}>
        <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true"><path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
        Create Connection
      </button>
    </div>

    <p class="foot">A native multi-database client.</p>
  </aside>

  <section class="list-pane">
    <div class="search-row">
      <div class="search">
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><circle cx="7" cy="7" r="4.5" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M11 11l3 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
        <input bind:value={query} placeholder="Search connections…" spellcheck="false" />
      </div>
      <button class="btn add" on:click={newConnection} title="New connection">
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>
      </button>
    </div>

    {#if error}<div class="banner err">{error}</div>{/if}

    {#if list.length === 0}
      <div class="empty">
        {#if $savedConnections.length === 0}
          <p class="empty-title">No connections yet</p>
          <p class="empty-sub">Create one to start browsing schemas and running SQL.</p>
          <button class="btn btn-primary" on:click={newConnection}>Create Connection</button>
        {:else}
          <p class="empty-title">No matches for “{query}”</p>
        {/if}
      </div>
    {:else}
      <ul class="conns">
        {#each rows as row (row.type === "header" ? "h:" + row.name : row.c.id)}
          {#if row.type === "header"}
            <li class="ghead">
              <button class="ghead-btn" on:click={() => toggleGroup(row.name)}>
                <span class="chev" class:open={row.open}>▸</span>
                <span class="gname">{row.name}</span>
                <span class="gcount">{row.count}</span>
              </button>
            </li>
          {:else}
            {@const c = row.c}
          <li>
            <button class="conn" on:click={() => open(c)} disabled={!!connectingId}>
              <span class="dot" style="--c: {statusVar(c.color)}" title={c.tag ?? ""}></span>
              <span class="cn-badge" style="--c: {dbKind(c.kind).color}">{dbKind(c.kind).abbr}</span>
              <span class="cn-text">
                <span class="cn-name">
                  {c.name}
                  {#if c.tag}<span class="cn-tag" style="--c: {statusVar(c.color)}">{c.tag}</span>{/if}
                </span>
                <span class="cn-sub">{subtitle(c)}</span>
              </span>
              {#if connectingId === c.id}
                <span class="spin" aria-label="Connecting"></span>
              {:else}
                <span class="cn-actions">
                  <button class="icon" title="Edit" on:click={(e) => edit(c, e)} aria-label="Edit">
                    <svg viewBox="0 0 16 16" width="13" height="13" aria-hidden="true"><path d="M11.5 2.5l2 2-7.5 7.5-2.6.6.6-2.6 7.5-7.5z" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/></svg>
                  </button>
                  <button class="icon danger" title="Delete" on:click={(e) => del(c, e)} aria-label="Delete">
                    <svg viewBox="0 0 16 16" width="13" height="13" aria-hidden="true"><path d="M3 4.5h10M6.5 4V3h3v1M5 4.5l.6 8h4.8l.6-8" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>
                  </button>
                </span>
              {/if}
            </button>
          </li>
          {/if}
        {/each}
      </ul>
    {/if}
  </section>
</div>

{#if picking}
  <DbPicker on:pick={onPick} on:close={() => (picking = false)} />
{/if}
{#if editing}
  {@const cfg = editing}
  <ConnectionForm
    config={editing}
    on:close={() => (editing = null)}
    on:connected={(e) => dispatch("connected", { id: e.detail, config: cfg })}
  />
{/if}

<style>
  .welcome {
    position: relative;
    height: 100vh;
    display: grid;
    grid-template-columns: 300px 1fr;
    background: var(--bg-content);
  }
  .dismiss {
    position: absolute; top: var(--s-5); right: var(--s-5); z-index: 1;
    width: 30px; height: 30px; border-radius: var(--r-sm);
    display: grid; place-items: center; color: var(--muted);
    background: var(--bg-elevated); border: 1px solid var(--border);
    transition: color var(--t-fast) var(--ease-out), background var(--t-fast) var(--ease-out);
  }
  .dismiss:hover { color: var(--ink); background: var(--bg-hover); }

  /* ── Brand panel ─────────────────────────────────────────── */
  .brand {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: var(--s-9) var(--s-8);
    background: var(--bg-panel);
    border-right: 1px solid var(--border);
  }
  .logo { margin-bottom: var(--s-5); }
  .brand h1 { margin: 0; font-size: 26px; font-weight: 700; letter-spacing: -0.025em; }
  .version { margin: 2px 0 0; color: var(--faint); font-size: 11.5px; }
  .actions { margin-top: var(--s-9); display: flex; flex-direction: column; gap: var(--s-2); width: 100%; }
  .action {
    display: flex; align-items: center; gap: var(--s-3);
    padding: var(--s-3) var(--s-4); border-radius: var(--r-sm);
    color: var(--ink-soft); font-size: 12.5px; font-weight: 500;
    transition: background var(--t-fast) var(--ease-out), color var(--t-fast) var(--ease-out);
  }
  .action:hover { background: var(--bg-elevated); color: var(--ink); }
  .action svg { color: var(--muted); }
  .foot { margin-top: auto; color: var(--faint); font-size: 11px; }

  /* ── List pane ───────────────────────────────────────────── */
  .list-pane { display: flex; flex-direction: column; padding: var(--s-7) var(--s-7) var(--s-6); min-width: 0; }
  .search-row { display: flex; gap: var(--s-3); margin-bottom: var(--s-5); }
  .search {
    flex: 1; display: flex; align-items: center; gap: var(--s-3);
    height: 32px; padding: 0 var(--s-4);
    background: var(--bg-elevated); border: 1px solid var(--border); border-radius: var(--r-md);
    color: var(--faint);
    transition: border-color var(--t-fast) var(--ease-out);
  }
  .search:focus-within { border-color: var(--accent); }
  .search input { flex: 1; background: none; border: none; outline: none; color: var(--ink); font: inherit; }
  .search input::placeholder { color: var(--faint); }
  .add { width: 32px; height: 32px; padding: 0; flex: none; }

  .conns { list-style: none; margin: 0; padding: 0; overflow-y: auto; flex: 1; display: flex; flex-direction: column; gap: 1px; }
  .ghead { margin-top: var(--s-2); }
  .ghead-btn { display: flex; align-items: center; gap: var(--s-2); width: 100%; padding: var(--s-1) var(--s-2); font-size: 10.5px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; color: var(--faint); background: none; border: none; }
  .ghead-btn:hover { color: var(--muted); }
  .chev { display: inline-block; font-size: 9px; transition: transform var(--t-fast) var(--ease-out); }
  .chev.open { transform: rotate(90deg); }
  .gcount { margin-left: auto; font-weight: 600; color: var(--faint); }
  .conn {
    width: 100%; display: flex; align-items: center; gap: var(--s-4);
    padding: var(--s-3) var(--s-3); border-radius: var(--r-md); text-align: left;
    transition: background var(--t-fast) var(--ease-out);
  }
  .conn:hover { background: var(--bg-elevated); }
  .conn:active { background: var(--bg-hover); }
  .conn:disabled { cursor: default; }
  .conn:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--focus-ring); }

  .dot { width: 9px; height: 9px; border-radius: 50%; background: var(--c); flex: none; box-shadow: 0 0 0 1px rgba(0,0,0,0.3) inset; }
  .cn-badge {
    width: 30px; height: 30px; border-radius: 50%; flex: none;
    display: grid; place-items: center; color: #fff; font-weight: 700; font-size: 12px;
    background: var(--c); box-shadow: inset 0 1px 0 rgba(255,255,255,0.22);
  }
  .cn-text { display: flex; flex-direction: column; min-width: 0; flex: 1; }
  .cn-name { display: flex; align-items: center; gap: var(--s-3); font-size: 13px; font-weight: 600; color: var(--ink); }
  .cn-tag {
    font-size: 10px; font-weight: 600; text-transform: lowercase;
    color: var(--c); border: 1px solid color-mix(in srgb, var(--c) 45%, transparent);
    border-radius: var(--r-xs); padding: 0 5px; line-height: 15px;
  }
  .cn-sub { font-size: 11.5px; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-family: var(--font-mono); }

  .cn-actions { display: flex; gap: 2px; margin-left: auto; opacity: 0; transition: opacity var(--t-fast) var(--ease-out); }
  .conn:hover .cn-actions { opacity: 1; }
  .icon {
    width: 24px; height: 24px; border-radius: var(--r-sm); display: grid; place-items: center; color: var(--muted);
    transition: background var(--t-fast) var(--ease-out), color var(--t-fast) var(--ease-out);
  }
  .icon:hover { background: var(--bg-active); color: var(--ink); }
  .icon.danger:hover { color: var(--danger); }

  .spin {
    width: 15px; height: 15px; margin-left: auto; border-radius: 50%; flex: none;
    border: 2px solid var(--border-strong); border-top-color: var(--accent);
    animation: spin 0.6s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  .empty { margin: auto; text-align: center; display: flex; flex-direction: column; align-items: center; gap: var(--s-3); color: var(--muted); }
  .empty-title { margin: 0; font-size: 14px; font-weight: 600; color: var(--ink-soft); }
  .empty-sub { margin: 0 0 var(--s-3); font-size: 12px; color: var(--muted); max-width: 30ch; }

  .banner { font-family: var(--font-mono); font-size: 11.5px; padding: var(--s-3) var(--s-4); border-radius: var(--r-sm); margin-bottom: var(--s-4); white-space: pre-wrap; }
  .banner.err { background: var(--danger-soft); color: var(--danger); }

  @media (prefers-reduced-motion: reduce) { .spin { animation-duration: 1ms; } }
</style>
