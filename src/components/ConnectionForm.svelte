<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import Modal from "./Modal.svelte";
  import { api } from "../lib/tauri";
  import { upsertConnection, resolvePassword } from "../lib/stores/connection";
  import { dbKind, STATUS_COLORS } from "../lib/dbKinds";
  import type { ConnectionConfig, StatusColor, TlsMode, SshProfile, SshRef, SshAuth } from "../lib/types";

  // Initial config — kind is preset by the picker; full config when editing.
  export let config: ConnectionConfig;

  const dispatch = createEventDispatcher<{ close: void; connected: string }>();

  const meta = dbKind(config.kind);
  $: isSqlite = config.kind === "sqlite";
  $: isMysql = config.kind === "mysql";
  $: sslModes = isMysql
    ? ["prefer", "require", "verify-ca", "verify-full", "disable"]
    : ["prefer", "require", "verify-ca", "verify-full", "allow", "disable"];

  let busy = false;
  let error: string | null = null;
  let testOk = false;

  // Local form state: password plaintext (used when kind === "plain" or "keychain").
  let plaintext = "";

  // Ensure config.password has a valid shape on mount (older persisted configs may omit it).
  if (!config.password || !config.password.kind) {
    config.password = { kind: "keychain" };
  }

  function onPasswordKindChange(e: Event) {
    const kind = (e.currentTarget as HTMLSelectElement).value;
    if (kind !== "plain" && kind !== "keychain") {
      plaintext = "";
    }
    switch (kind) {
      case "plain":
        config.password = { kind: "plain" };
        break;
      case "keychain":
        config.password = { kind: "keychain" };
        break;
      case "env":
        config.password = { kind: "env", name: (config.password as { kind: "env"; name: string }).name ?? "" };
        break;
      case "onepassword":
        config.password = {
          kind: "onepassword",
          item: (config.password as { kind: "onepassword"; item: string; field: string }).item ?? "",
          field: (config.password as { kind: "onepassword"; item: string; field: string }).field ?? "password",
        };
        break;
      case "vault":
        config.password = {
          kind: "vault",
          path: (config.password as { kind: "vault"; path: string; field: string }).path ?? "",
          field: (config.password as { kind: "vault"; path: string; field: string }).field ?? "password",
        };
        break;
      case "aws-sm":
        config.password = {
          kind: "aws-sm",
          arn: (config.password as { kind: "aws-sm"; arn: string; region: string }).arn ?? "",
          region: (config.password as { kind: "aws-sm"; arn: string; region: string }).region ?? "us-east-1",
        };
        break;
    }
    config = config; // trigger Svelte reactivity
  }

  function validatePassword(): string | null {
    const p = config.password;
    if (p.kind === "onepassword") {
      if (!p.item?.trim()) return "1Password: Item is required.";
      if (!p.field?.trim()) return "1Password: Field is required.";
    }
    if (p.kind === "vault") {
      if (!p.path?.trim()) return "Vault: Path is required.";
      if (!p.field?.trim()) return "Vault: Field is required.";
    }
    if (p.kind === "aws-sm") {
      if (!p.arn?.trim()) return "AWS: Secret ARN is required.";
      if (!p.region?.trim()) return "AWS: Region is required.";
    }
    return null;
  }

  // Narrowed reactive references for password sub-configs — needed because
  // Svelte's `bind:value` inside `{#if}` blocks can't narrow discriminated unions.
  $: envPw = config.password.kind === "env" ? config.password : null;
  $: opPw = config.password.kind === "onepassword" ? config.password : null;
  $: vaultPw = config.password.kind === "vault" ? config.password : null;
  $: awsPw = config.password.kind === "aws-sm" ? config.password : null;

  // SSL / TLS local state — derived from config.tls on mount.
  let enableSsl = config.tls != null && config.tls.mode !== "disable";
  let sslModeChoice: TlsMode = config.tls?.mode ?? "prefer";
  let caPath = config.tls?.ca_path ?? "";
  let certPath = config.tls?.cert_path ?? "";
  let keyPath = config.tls?.key_path ?? "";

  // SSH local state — derived from config.ssh on mount.
  let sshHost = (config.ssh?.kind === "inline" ? config.ssh.value.host : "") ?? "";
  let sshPort = (config.ssh?.kind === "inline" ? config.ssh.value.port : 22) ?? 22;
  let sshUser = (config.ssh?.kind === "inline" ? config.ssh.value.user : "") ?? "";
  let sshKey = (config.ssh?.kind === "inline" && config.ssh.value.auth.kind === "key-file" ? config.ssh.value.auth.path : "") ?? "";

  // Three-state SSH radio: Off / Use saved profile / Inline.
  type SshMode = "off" | "profile" | "inline";

  function deriveSshMode(ssh: SshRef | null | undefined): SshMode {
    if (!ssh) return "off";
    return ssh.kind === "profile" ? "profile" : "inline";
  }

  let sshMode: SshMode = deriveSshMode(config.ssh);
  let selectedProfileId: string | null =
    config.ssh?.kind === "profile" ? config.ssh.value : null;
  let existingInlineId: string | null =
    config.ssh?.kind === "inline" ? config.ssh.value.id : null;
  let profiles: SshProfile[] = [];

  onMount(async () => {
    try {
      profiles = await api.listSshProfiles();
    } catch {
      profiles = [];
    }
  });

  function assembleSsh(): SshRef | null {
    if (sshMode === "off") return null;
    if (sshMode === "profile" && selectedProfileId) {
      return { kind: "profile", value: selectedProfileId };
    }
    if (sshMode === "inline") {
      const auth: SshAuth = sshKey
        ? { kind: "key-file", path: sshKey, passphrase: null }
        : { kind: "agent" };
      return {
        kind: "inline",
        value: {
          id: existingInlineId ?? crypto.randomUUID(),
          name: "inline",
          host: sshHost.trim(),
          port: sshPort || 22,
          user: sshUser.trim(),
          auth,
          jump: null,
        },
      };
    }
    return null;
  }

  function openSshProfilesInSettings() {
    // Fallback: user has to click Settings themselves and pick SSH Profiles.
    // A future iteration wires this to a Settings-open event with tab arg.
    alert("Open Settings → SSH Profiles to manage profiles.");
  }

  async function browseSshKey() {
    try {
      const picked = await open({ multiple: false, directory: false });
      if (typeof picked === "string") sshKey = picked;
    } catch {
      // silently ignore
    }
  }

  // Ensure safety has a default when not set (e.g. older persisted configs).
  $: if (config && !config.safety) config.safety = "confirm-destructive";

  // Guard: profile mode selected but no profile chosen (or profiles unavailable).
  $: sshProfileIncomplete = sshMode === "profile" && !selectedProfileId;

  // Load plaintext from keychain when editing an existing connection.
  resolvePassword(config).then((p) => { if (p) plaintext = p; });

  function buildConfig(): ConnectionConfig {
    return {
      ...config,
      password: config.password,
      tls: enableSsl
        ? {
            mode: sslModeChoice,
            ca_path: caPath || null,
            cert_path: certPath || null,
            key_path: keyPath || null,
          }
        : null,
      ssh: assembleSsh(),
    };
  }

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
      const cfg = buildConfig();
      const id = await api.connect(cfg);
      await api.disconnect(id);
      testOk = true;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function save() {
    const pwErr = validatePassword();
    if (pwErr) {
      error = pwErr;
      return;
    }
    const cfg = buildConfig();
    await upsertConnection(cfg, config.password.kind === "keychain" ? plaintext || undefined : undefined);
    dispatch("close");
  }

  async function connect() {
    busy = true;
    error = null;
    try {
      const cfg = buildConfig();
      const id = await api.connect(cfg);
      await upsertConnection(cfg, config.password.kind === "keychain" ? plaintext || undefined : undefined);
      dispatch("connected", id);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function pickSqliteFile() {
    try {
      const selected = await open({
        filters: [
          {
            name: "SQLite",
            extensions: ["db", "sqlite", "sqlite3"],
          },
          {
            name: "All Files",
            extensions: ["*"],
          },
        ],
      });

      if (typeof selected === "string") {
        config.file_path = selected;
      }
    } catch (e) {
      error = `Failed to pick file: ${e}`;
    }
  }
</script>

<Modal title="{meta.label} Connection" width="480px" on:close={() => dispatch("close")}>
  <div class="form">
    <label class="row">
      <span class="lbl">Name</span>
      <input class="field" bind:value={config.name} placeholder="My database" />
    </label>
    <label class="row">
      <span class="lbl">Group</span>
      <input class="field" bind:value={config.group} placeholder="folder (optional, e.g. Production)" />
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

    <label class="row">
      <span class="lbl">Safety</span>
      <select class="field" bind:value={config.safety}>
        <option value="off">Off — no guards</option>
        <option value="warn">Warn — banner only</option>
        <option value="confirm-destructive">Confirm destructive (default)</option>
        <option value="confirm-writes">Confirm writes</option>
        <option value="confirm-ddl">Confirm DDL</option>
        <option value="read-only">Read-only</option>
      </select>
    </label>

    {#if isSqlite}
      <label class="row">
        <span class="lbl">File path</span>
        <div class="filepath-row">
          <input class="field" bind:value={config.file_path} placeholder="/path/to/db.sqlite" />
          <button class="btn btn-icon" type="button" on:click={pickSqliteFile} title="Browse for SQLite file">
            Browse
          </button>
        </div>
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

      <div class="row">
        <span class="lbl">Password</span>
        <select class="field" value={config.password.kind} on:change={onPasswordKindChange}>
          <option value="plain">Plaintext</option>
          <option value="keychain">OS Keychain</option>
          <option value="env">Environment variable</option>
          <option value="onepassword">1Password</option>
          <option value="vault">Vault</option>
          <option value="aws-sm">AWS Secrets Manager</option>
        </select>
      </div>

      {#if config.password.kind === "plain"}
        <div class="field-row">
          <label for="pw-plain">Password</label>
          <input id="pw-plain" class="field" type="password" bind:value={plaintext} placeholder="•••••••" />
        </div>
      {/if}

      {#if config.password.kind === "keychain"}
        <div class="field-row">
          <label for="pw-keychain">Password</label>
          <input id="pw-keychain" class="field" type="password" bind:value={plaintext} placeholder="•••••••" />
        </div>
        <p class="hint">Stored securely in your OS keychain — never written to disk.</p>
      {/if}

      {#if config.password.kind === "env" && envPw}
        <div class="field-row">
          <label for="env-name">Variable name</label>
          <input id="env-name" class="field" type="text" bind:value={envPw.name} placeholder="PGPASSWORD" required />
        </div>
      {/if}

      {#if config.password.kind === "onepassword" && opPw}
        <div class="field-row">
          <label for="op-item">Item</label>
          <input id="op-item" class="field" type="text" bind:value={opPw.item} placeholder="Postgres Prod (name or UUID)" required />
        </div>
        <div class="field-row">
          <label for="op-field">Field</label>
          <input id="op-field" class="field" type="text" bind:value={opPw.field} placeholder="password" required />
        </div>
        <p class="hint">Requires the <code>op</code> CLI installed and signed in.</p>
      {/if}

      {#if config.password.kind === "vault" && vaultPw}
        <div class="field-row">
          <label for="vault-path">Path</label>
          <input id="vault-path" class="field" type="text" bind:value={vaultPw.path} placeholder="secret/data/prod/pg" required />
        </div>
        <div class="field-row">
          <label for="vault-field">Field</label>
          <input id="vault-field" class="field" type="text" bind:value={vaultPw.field} placeholder="password" required />
        </div>
        <p class="hint">Requires the <code>vault</code> CLI with <code>VAULT_ADDR</code> and <code>VAULT_TOKEN</code> set.</p>
      {/if}

      {#if config.password.kind === "aws-sm" && awsPw}
        <div class="field-row">
          <label for="aws-arn">Secret ARN</label>
          <input id="aws-arn" class="field" type="text" bind:value={awsPw.arn} placeholder="arn:aws:secretsmanager:us-east-1:…:secret:name" required />
        </div>
        <div class="field-row">
          <label for="aws-region">Region</label>
          <input id="aws-region" class="field" type="text" bind:value={awsPw.region} placeholder="us-east-1" required />
        </div>
        <p class="hint">Requires the <code>aws</code> CLI with credentials on PATH.</p>
      {/if}

      <label class="row">
        <span class="lbl">Database</span>
        <input class="field" bind:value={config.database} placeholder="database name" />
      </label>

      <label class="row check">
        <input type="checkbox" bind:checked={enableSsl} />
        <span>Use SSL</span>
      </label>

      {#if enableSsl}
        <label class="row">
          <span class="lbl">SSL mode</span>
          <select class="field" bind:value={sslModeChoice}>
            {#each sslModes as m (m)}<option value={m}>{m}</option>{/each}
          </select>
        </label>
        <label class="row">
          <span class="lbl">CA cert</span>
          <input class="field" bind:value={caPath} placeholder="/path/to/ca.pem (optional)" />
        </label>
        {#if !isMysql}
          <label class="row">
            <span class="lbl">Client cert</span>
            <input class="field" bind:value={certPath} placeholder="/path/to/client.crt (optional)" />
          </label>
          <label class="row">
            <span class="lbl">Client key</span>
            <input class="field" bind:value={keyPath} placeholder="/path/to/client.key (optional)" />
          </label>
        {/if}
      {/if}

      <div class="ssh-section">
        <div class="ssh-mode">
          <label class="ssh-radio"><input type="radio" bind:group={sshMode} value="off" /> Off</label>
          {#if profiles.length > 0}
            <label class="ssh-radio"><input type="radio" bind:group={sshMode} value="profile" /> Use saved profile</label>
          {/if}
          <label class="ssh-radio"><input type="radio" bind:group={sshMode} value="inline" /> Configure inline</label>
        </div>

        {#if sshMode === "profile"}
          {#if profiles.length === 0}
            <div class="banner ssh-warn">⚠ SSH profile list unavailable — save disabled to prevent data loss.</div>
          {:else}
            <label class="row">
              <span class="lbl">Profile</span>
              <select class="field" bind:value={selectedProfileId}>
                <option value={null}>— select a profile —</option>
                {#each profiles as p (p.id)}
                  <option value={p.id}>{p.name} ({p.user}@{p.host}:{p.port})</option>
                {/each}
              </select>
            </label>
            <p class="ssh-note">
              <button type="button" class="link-btn" on:click={openSshProfilesInSettings}>
                Manage profiles in Settings…
              </button>
            </p>
          {/if}
        {/if}

        {#if sshMode === "inline"}
          <label class="row">
            <span class="lbl">SSH host</span>
            <input class="field" bind:value={sshHost} placeholder="bastion.example.com" />
          </label>
          <label class="row">
            <span class="lbl">SSH port</span>
            <input class="field port" type="number" bind:value={sshPort} placeholder="22" />
          </label>
          <label class="row">
            <span class="lbl">SSH user</span>
            <input class="field" bind:value={sshUser} placeholder="ubuntu" />
          </label>
          <div class="row">
            <span class="lbl">Private key</span>
            <div class="filepath-row">
              <input class="field" bind:value={sshKey} placeholder="/Users/me/.ssh/id_rsa (blank = agent)" />
              <button type="button" class="btn btn-icon" on:click={browseSshKey}>Browse…</button>
            </div>
          </div>
          <p class="ssh-note">Key/agent auth only. The DB host/port above are reached through the tunnel.</p>
        {/if}
      </div>
    {/if}

    {#if error}
      <div class="banner err">{error}</div>
    {:else if testOk}
      <div class="banner ok">Connection successful.</div>
    {/if}
  </div>

  <svelte:fragment slot="footer">
    <button class="btn" on:click={test} disabled={busy || sshProfileIncomplete}>
      {busy ? "Testing…" : "Test"}
    </button>
    <div class="spacer"></div>
    <button
      class="btn"
      on:click={save}
      disabled={busy || sshProfileIncomplete}
      title={sshProfileIncomplete ? "Select an SSH profile or switch to Off/Inline" : undefined}
    >Save</button>
    <button
      class="btn btn-primary"
      on:click={connect}
      disabled={busy || sshProfileIncomplete}
      title={sshProfileIncomplete ? "Select an SSH profile or switch to Off/Inline" : undefined}
    >
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

  .filepath-row { display: flex; align-items: center; gap: var(--s-3); }
  .filepath-row .field { flex: 1; min-width: 0; }
  .filepath-row .btn-icon { flex: none; padding: var(--s-2) var(--s-3); font-size: 12px; }

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

  .field-row { display: grid; grid-template-columns: 92px 1fr; align-items: center; gap: var(--s-5); }
  .field-row label { font-size: 12px; color: var(--muted); text-align: right; }

  .ssh-section { display: flex; flex-direction: column; gap: var(--s-4); }
  .ssh-mode { display: flex; align-items: center; gap: var(--s-4); }
  .ssh-radio { display: flex; align-items: center; gap: var(--s-2); font-size: 12.5px; color: var(--ink-soft); cursor: pointer; }
  .ssh-radio input { margin: 0; accent-color: var(--accent); }
  .link-btn { background: none; border: none; padding: 0; color: var(--accent); font-size: 11px; cursor: pointer; text-decoration: underline; }
  .link-btn:hover { opacity: 0.8; }
  .ssh-note { margin: calc(-1 * var(--s-2)) 0 0; font-size: 11px; color: var(--faint); line-height: 1.45; }
  .banner {
    font-family: var(--font-mono); font-size: 11.5px; line-height: 1.5;
    padding: var(--s-3) var(--s-4); border-radius: var(--r-sm);
    white-space: pre-wrap; word-break: break-word;
  }
  .banner.err { background: var(--danger-soft); color: var(--danger); }
  .banner.ok { background: rgba(48, 209, 88, 0.13); color: var(--success); }
  .banner.ssh-warn { background: rgba(255, 190, 50, 0.15); color: var(--warn, #b87a00); font-family: inherit; font-size: 12px; }

  .spacer { flex: 1; }
</style>
