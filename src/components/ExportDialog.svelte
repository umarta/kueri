<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import { api } from "../lib/tauri";
  import type { ConnectionConfig } from "../lib/types";

  export let cfg: ConnectionConfig;

  const dispatch = createEventDispatcher<{ close: void }>();

  let format: "plain" | "custom" = "plain";
  let contents: "all" | "schema" | "data" = "all";
  let busy = false;
  let result: { ok: boolean; msg: string } | null = null;

  $: isPg = cfg.kind === "postgres";
  $: isMysql = cfg.kind === "mysql";
  $: isSqlite = cfg.kind === "sqlite";

  async function run() {
    result = null;
    const ext = isSqlite ? "sqlite" : format === "custom" ? "dump" : "sql";
    const filterName = isSqlite ? "SQLite database" : format === "custom" ? "Postgres dump" : "SQL";
    const path = await save({
      defaultPath: `${cfg.database || "database"}.${ext}`,
      filters: [{ name: filterName, extensions: [ext] }],
    });
    if (!path) return; // cancelled
    busy = true;
    try {
      const msg = await api.pgExport(cfg, path, format, contents);
      result = { ok: true, msg: `${msg}\nSaved to ${path}` };
    } catch (e) {
      result = { ok: false, msg: (e as { message?: string })?.message ?? String(e) };
    } finally {
      busy = false;
    }
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
<div class="backdrop" on:click|self={() => dispatch("close")}>
  <div class="panel" role="dialog" aria-label="Export database">
    <header class="head">
      <span class="title">Export database</span>
      <span class="db">{cfg.database}</span>
    </header>

    <div class="body">
      {#if isPg}
        <div class="ed-field">
          <span class="lbl">Format</span>
          <div class="seg">
            <button class:active={format === "plain"} on:click={() => (format = "plain")}>Plain SQL (.sql)</button>
            <button class:active={format === "custom"} on:click={() => (format = "custom")}>Custom (.dump)</button>
          </div>
        </div>
      {/if}
      {#if !isSqlite}
        <div class="ed-field">
          <span class="lbl">Contents</span>
          <div class="seg">
            <button class:active={contents === "all"} on:click={() => (contents = "all")}>Schema + data</button>
            <button class:active={contents === "schema"} on:click={() => (contents = "schema")}>Schema only</button>
            <button class:active={contents === "data"} on:click={() => (contents = "data")}>Data only</button>
          </div>
        </div>
      {/if}
      <p class="hint">
        {#if isPg}Runs <code>pg_dump</code> (PostgreSQL client tools).{:else if isMysql}Runs <code>mysqldump</code> (MySQL client tools).{:else}Copies the SQLite database file.{/if}
      </p>

      {#if result}
        <pre class="result" class:err={!result.ok}>{result.msg}</pre>
      {/if}
    </div>

    <footer class="foot">
      <button class="btn ghost" on:click={() => dispatch("close")}>{result?.ok ? "Done" : "Cancel"}</button>
      <button class="btn primary" on:click={run} disabled={busy}>
        {busy ? "Exporting…" : "Choose file & export"}
      </button>
    </footer>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; z-index: var(--z-modal); display: grid; place-items: center; background: rgba(0, 0, 0, 0.45); }
  .panel { width: min(520px, 92vw); display: flex; flex-direction: column; background: var(--bg-panel); border: 1px solid var(--border-strong); border-radius: var(--r-lg); box-shadow: var(--shadow-modal); overflow: hidden; }
  .head { display: flex; align-items: baseline; gap: var(--s-3); padding: var(--s-4) var(--s-5); border-bottom: 1px solid var(--hairline); }
  .title { font-size: 13px; font-weight: 600; color: var(--ink); }
  .db { font-size: 12px; color: var(--muted); font-family: var(--font-mono); }

  .body { padding: var(--s-5); display: flex; flex-direction: column; gap: var(--s-4); }
  .ed-field { display: flex; flex-direction: column; gap: var(--s-2); }
  .lbl { font-size: 11px; font-weight: 600; color: var(--muted); text-transform: uppercase; letter-spacing: 0.04em; }
  .seg { display: flex; gap: var(--s-2); flex-wrap: wrap; }
  .seg button { height: 30px; padding: 0 var(--s-4); border-radius: var(--r-sm); font: inherit; font-size: 12.5px; color: var(--ink-soft); background: var(--bg-content); border: 1px solid var(--border); }
  .seg button:hover { border-color: var(--border-strong); }
  .seg button.active { border-color: var(--accent); color: var(--ink); background: color-mix(in srgb, var(--accent) 14%, var(--bg-content)); }

  .hint { margin: 0; font-size: 12px; color: var(--muted); }
  .hint code { font-family: var(--font-mono); color: var(--ink-soft); background: var(--bg-elevated); padding: 0 4px; border-radius: var(--r-xs); }

  .result { margin: 0; padding: var(--s-3); border-radius: var(--r-sm); background: var(--bg-content); border: 1px solid var(--border); color: var(--ink-soft); font-family: var(--font-mono); font-size: 11.5px; white-space: pre-wrap; max-height: 180px; overflow: auto; }
  .result.err { color: var(--danger); border-color: color-mix(in srgb, var(--danger) 30%, transparent); background: var(--danger-soft); }

  .foot { display: flex; justify-content: flex-end; gap: var(--s-3); padding: var(--s-4) var(--s-5); border-top: 1px solid var(--hairline); }
  .btn { height: 30px; padding: 0 var(--s-5); border-radius: var(--r-sm); font: inherit; font-size: 12.5px; font-weight: 600; border: 1px solid transparent; }
  .btn.ghost { background: transparent; border-color: var(--border); color: var(--ink-soft); }
  .btn.ghost:hover { background: var(--bg-elevated); }
  .btn.primary { background: var(--accent); color: var(--accent-ink); }
  .btn.primary:hover:not(:disabled) { filter: brightness(1.05); }
  .btn.primary:disabled { opacity: 0.5; }
</style>
