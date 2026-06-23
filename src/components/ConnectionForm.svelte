<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import Modal from "./Modal.svelte";
  import { api } from "../lib/tauri";
  import { upsertConnection } from "../lib/stores/connection";
  import { dbKind, STATUS_COLORS } from "../lib/dbKinds";
  import type { ConnectionConfig, StatusColor } from "../lib/types";

  // Initial config — kind is preset by the picker; full config when editing.
  export let config: ConnectionConfig;

  const dispatch = createEventDispatcher<{ close: void; connected: string }>();

  const meta = dbKind(config.kind);
  $: isSqlite = config.kind === "sqlite";
  $: isMysql = config.kind === "mysql";
  $: sslModes = isMysql
    ? ["PREFERRED", "REQUIRED", "VERIFY_CA", "VERIFY_IDENTITY", "DISABLED"]
    : ["prefer", "require", "verify-ca", "verify-full", "allow", "disable"];

  let busy = false;
  let error: string | null = null;
  let testOk = false;

  function pickColor(c: StatusColor) {
    config.color = c;
    // Mirror TablePlus: choosing an environment color suggests its label,
    // but the tag stays editable.
    const suggested = STATUS_COLORS.find((s) => s.value === c)?.label ?? "";
    if (!config.tag && ["Local", "Staging", "Production"].includes(suggested))
      config.tag = suggested.toLowerCase();
  }

  async function test() {
    busy = true;
    error = null;
    testOk = false;
    try {
      const id = await api.connect(config);
      await api.disconnect(id);
      testOk = true;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function save() {
    await upsertConnection(config);
    dispatch("close");
  }

  async function connect() {
    busy = true;
    error = null;
    try {
      const id = await api.connect(config);
      await upsertConnection(config);
      dispatch("connected", id);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<Modal title="{meta.label} Connection" width="480px" on:close={() => dispatch("close")}>
  <div class="form">
    <label class="row">
      <span class="lbl">Name</span>
      <input class="field" bind:value={config.name} placeholder="My database" />
    </label>

    <div class="row">
      <span class="lbl">Environment</span>
      <div class="env">
        <div class="dots" role="radiogroup" aria-label="Environment color">
          {#each STATUS_COLORS as s (s.value)}
            <button
              type="button"
              class="dot"
              class:on={config.color === s.value}
              style="--c: {s.var}"
              role="radio"
              aria-checked={config.color === s.value}
              aria-label={s.label}
              title={s.label}
              on:click={() => pickColor(s.value)}
            ></button>
          {/each}
        </div>
        <input class="field tag" bind:value={config.tag} placeholder="tag (e.g. staging)" />
      </div>
    </div>

    {#if isSqlite}
      <label class="row">
        <span class="lbl">File path</span>
        <input class="field" bind:value={config.file_path} placeholder="/path/to/db.sqlite" />
      </label>
    {:else}
      <div class="row">
        <span class="lbl">Host</span>
        <div class="hostport">
          <input class="field" bind:value={config.host} placeholder="localhost" />
          <span class="sublbl">Port</span>
          <input class="field port" type="number" bind:value={config.port} />
        </div>
      </div>

      <label class="row">
        <span class="lbl">User</span>
        <input class="field" bind:value={config.user} placeholder="user" />
      </label>

      <label class="row">
        <span class="lbl">Password</span>
        <input class="field" type="password" bind:value={config.password} placeholder="•••••••" />
      </label>
      <p class="hint">Stored securely in your OS keychain — never written to disk.</p>

      <label class="row">
        <span class="lbl">Database</span>
        <input class="field" bind:value={config.database} placeholder="database name" />
      </label>

      <label class="row check">
        <input type="checkbox" bind:checked={config.ssl} />
        <span>Use SSL</span>
      </label>

      {#if config.ssl}
        <label class="row">
          <span class="lbl">SSL mode</span>
          <select class="field" bind:value={config.ssl_mode}>
            <option value="">(default)</option>
            {#each sslModes as m (m)}<option value={m}>{m}</option>{/each}
          </select>
        </label>
        <label class="row">
          <span class="lbl">CA cert</span>
          <input class="field" bind:value={config.ssl_ca} placeholder="/path/to/ca.pem (optional)" />
        </label>
        {#if !isMysql}
          <label class="row">
            <span class="lbl">Client cert</span>
            <input class="field" bind:value={config.ssl_cert} placeholder="/path/to/client.crt (optional)" />
          </label>
          <label class="row">
            <span class="lbl">Client key</span>
            <input class="field" bind:value={config.ssl_key} placeholder="/path/to/client.key (optional)" />
          </label>
        {/if}
      {/if}

      <label class="row check">
        <input type="checkbox" bind:checked={config.ssh_enabled} />
        <span>Connect via SSH tunnel</span>
      </label>
      {#if config.ssh_enabled}
        <label class="row">
          <span class="lbl">SSH host</span>
          <input class="field" bind:value={config.ssh_host} placeholder="bastion.example.com" />
        </label>
        <label class="row">
          <span class="lbl">SSH port</span>
          <input class="field port" type="number" bind:value={config.ssh_port} placeholder="22" />
        </label>
        <label class="row">
          <span class="lbl">SSH user</span>
          <input class="field" bind:value={config.ssh_user} placeholder="ubuntu" />
        </label>
        <label class="row">
          <span class="lbl">Private key</span>
          <input class="field" bind:value={config.ssh_key} placeholder="~/.ssh/id_ed25519 (or blank for agent)" />
        </label>
        <p class="ssh-note">Key/agent auth only. The DB host/port above are reached through the tunnel.</p>
      {/if}
    {/if}

    {#if error}
      <div class="banner err">{error}</div>
    {:else if testOk}
      <div class="banner ok">Connection successful.</div>
    {/if}
  </div>

  <svelte:fragment slot="footer">
    <button class="btn" on:click={test} disabled={busy}>
      {busy ? "Testing…" : "Test"}
    </button>
    <div class="spacer"></div>
    <button class="btn" on:click={save} disabled={busy}>Save</button>
    <button class="btn btn-primary" on:click={connect} disabled={busy}>
      {busy ? "Connecting…" : "Connect"}
    </button>
  </svelte:fragment>
</Modal>

<style>
  .form { display: flex; flex-direction: column; gap: var(--s-4); }
  .row { display: grid; grid-template-columns: 92px 1fr; align-items: center; gap: var(--s-5); }
  .lbl { font-size: 12px; color: var(--muted); text-align: right; }

  .hostport { display: flex; align-items: center; gap: var(--s-3); }
  .hostport .field:first-child { flex: 1; min-width: 0; }
  .sublbl { font-size: 12px; color: var(--faint); }
  .hostport .port { width: 76px; flex: none; }

  .env { display: flex; align-items: center; gap: var(--s-4); }
  .dots { display: flex; gap: var(--s-2); }
  .dot {
    width: 18px; height: 18px; border-radius: 50%;
    background: var(--c);
    border: 2px solid transparent;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.3) inset;
    transition: transform var(--t-fast) var(--ease-out), box-shadow var(--t-fast) var(--ease-out);
  }
  .dot:hover { transform: scale(1.12); }
  .dot.on { box-shadow: 0 0 0 2px var(--bg-panel), 0 0 0 3.5px var(--c); }
  .tag { max-width: 180px; }

  .check { grid-template-columns: 92px 1fr; }
  .check span { font-size: 12.5px; color: var(--ink-soft); }
  .check input { margin: 0; justify-self: start; width: 15px; height: 15px; accent-color: var(--accent); }

  .hint { margin: -2px 0 0; padding-left: calc(92px + var(--s-5)); font-size: 11px; color: var(--faint); }

  .ssh-note { margin: calc(-1 * var(--s-2)) 0 0; font-size: 11px; color: var(--faint); line-height: 1.45; }
  .banner {
    font-family: var(--font-mono); font-size: 11.5px; line-height: 1.5;
    padding: var(--s-3) var(--s-4); border-radius: var(--r-sm);
    white-space: pre-wrap; word-break: break-word;
  }
  .banner.err { background: var(--danger-soft); color: var(--danger); }
  .banner.ok { background: rgba(48, 209, 88, 0.13); color: var(--success); }

  .spacer { flex: 1; }
</style>
