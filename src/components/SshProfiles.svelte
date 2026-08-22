<script lang="ts">
    import { onMount } from "svelte";
    import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
    import { api } from "../lib/tauri";
    import type { SshProfile, SshAuth } from "../lib/types";

    let profiles: SshProfile[] = [];
    let loading = true;
    let error: string | null = null;

    // Edit-form state (null = list view, otherwise the profile being edited)
    type FormState = {
        id: string;           // uuid; new profile generates one
        name: string;
        host: string;
        port: number;
        user: string;
        auth: "key-file" | "agent";
        keyPath: string;
        passphraseMode: "none" | "keychain";
    };
    let form: FormState | null = null;
    let formIsNew = false;
    let formError: string | null = null;

    // Delete dialog state
    let deleting: { profile: SshProfile; dependents: string[] } | null = null;

    onMount(refresh);

    async function refresh() {
        loading = true;
        error = null;
        try {
            profiles = await api.listSshProfiles();
        } catch (e) {
            error = (e as { message?: string })?.message ?? String(e);
        } finally {
            loading = false;
        }
    }

    function newProfile() {
        form = {
            id: crypto.randomUUID(),
            name: "",
            host: "",
            port: 22,
            user: "",
            auth: "agent",
            keyPath: "",
            passphraseMode: "none",
        };
        formIsNew = true;
        formError = null;
    }

    function editProfile(p: SshProfile) {
        const keyFileAuth = p.auth.kind === "key-file" ? p.auth : null;
        form = {
            id: p.id,
            name: p.name,
            host: p.host,
            port: p.port,
            user: p.user,
            auth: keyFileAuth ? "key-file" : "agent",
            keyPath: keyFileAuth ? keyFileAuth.path : "",
            passphraseMode:
                keyFileAuth && keyFileAuth.passphrase?.kind === "keychain" ? "keychain" : "none",
        };
        formIsNew = false;
        formError = null;
    }

    function cancelForm() {
        form = null;
        formError = null;
    }

    async function browseKey() {
        const picked = await openFileDialog({ multiple: false, directory: false });
        if (typeof picked === "string" && form) form.keyPath = picked;
    }

    function assembleAuth(f: FormState): SshAuth {
        if (f.auth === "agent") return { kind: "agent" };
        const passphrase =
            f.passphraseMode === "keychain" ? ({ kind: "keychain" } as const) : null;
        return { kind: "key-file", path: f.keyPath, passphrase };
    }

    async function saveForm() {
        if (!form) return;
        formError = null;
        const payload: SshProfile = {
            id: form.id,
            name: form.name.trim(),
            host: form.host.trim(),
            port: form.port || 22,
            user: form.user.trim(),
            auth: assembleAuth(form),
            jump: null,
        };
        if (!payload.name || !payload.host || !payload.user) {
            formError = "Name, host, and user are required.";
            return;
        }
        if (payload.auth.kind === "key-file" && !payload.auth.path) {
            formError = "Key file path is required.";
            return;
        }
        try {
            await api.saveSshProfile(payload);
            form = null;
            await refresh();
        } catch (e) {
            formError = (e as { message?: string })?.message ?? String(e);
        }
    }

    async function requestDelete(p: SshProfile) {
        try {
            const dependents = await api.listSshProfileDependents(p.id);
            deleting = { profile: p, dependents };
        } catch (e) {
            error = (e as { message?: string })?.message ?? String(e);
        }
    }

    async function confirmDelete() {
        if (!deleting) return;
        const snap = deleting;
        try {
            await api.deleteSshProfile(snap.profile.id);
            deleting = null;
            await refresh();
        } catch (e) {
            // Race case: dependent added between listDependents and delete.
            // Refetch dependents so the dialog reflects reality.
            const dependents = await api.listSshProfileDependents(snap.profile.id);
            deleting = { profile: snap.profile, dependents };
        }
    }
</script>

{#if !form}
    <div class="header">
        <h2>SSH Profiles</h2>
        <button class="btn" on:click={newProfile}>+ New</button>
    </div>

    {#if error}
        <div class="err">{error}</div>
    {/if}

    {#if loading}
        <div class="loading">Loading…</div>
    {:else if profiles.length === 0}
        <div class="empty">No SSH profiles yet. <button class="btn-link" on:click={newProfile}>+ New profile</button></div>
    {:else}
        <ul class="profiles">
            {#each profiles as p (p.id)}
                <li>
                    <button class="row" on:click={() => editProfile(p)}>
                        <span class="name">{p.name}</span>
                        <span class="target">{p.user}@{p.host}:{p.port}</span>
                    </button>
                    <button class="delete" on:click={() => requestDelete(p)} title="Delete">✕</button>
                </li>
            {/each}
        </ul>
    {/if}
{:else}
    <div class="header">
        <h2>{formIsNew ? "New SSH Profile" : "Edit SSH Profile"}</h2>
    </div>

    <label class="field-row">
        <span class="lbl">Name</span>
        <input class="field" bind:value={form.name} placeholder="bastion-prod" />
    </label>
    <label class="field-row">
        <span class="lbl">Host</span>
        <input class="field" bind:value={form.host} placeholder="10.0.1.4" />
    </label>
    <label class="field-row">
        <span class="lbl">Port</span>
        <input class="field field-short" type="number" bind:value={form.port} placeholder="22" />
    </label>
    <label class="field-row">
        <span class="lbl">User</span>
        <input class="field" bind:value={form.user} placeholder="ubuntu" />
    </label>

    <fieldset>
        <legend>Auth</legend>
        <label class="radio-row"><input type="radio" bind:group={form.auth} value="key-file" /> Private key file</label>
        <label class="radio-row"><input type="radio" bind:group={form.auth} value="agent" /> SSH agent</label>
    </fieldset>

    {#if form.auth === "key-file"}
        <label class="field-row">
            <span class="lbl">Path</span>
            <input class="field" bind:value={form.keyPath} placeholder="/Users/me/.ssh/id_rsa" />
            <button class="btn btn-browse" on:click={browseKey}>Browse…</button>
        </label>
        <fieldset>
            <legend>Passphrase</legend>
            <label class="radio-row"><input type="radio" bind:group={form.passphraseMode} value="none" /> None</label>
            <label class="radio-row"><input type="radio" bind:group={form.passphraseMode} value="keychain" /> From OS keychain</label>
        </fieldset>
    {/if}

    {#if formError}
        <div class="err">{formError}</div>
    {/if}

    <div class="actions">
        <button class="btn" on:click={cancelForm}>Cancel</button>
        <button class="btn btn-primary" on:click={saveForm}>Save</button>
    </div>
{/if}

{#if deleting}
    <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
    <div class="backdrop" on:click|self={() => (deleting = null)}>
        <div class="modal" role="dialog" aria-modal="true">
            <h3>Delete SSH profile "{deleting.profile.name}"?</h3>
            {#if deleting.dependents.length > 0}
                <p>This profile is used by the following connections:</p>
                <ul class="dep-list">
                    {#each deleting.dependents as name}
                        <li>{name}</li>
                    {/each}
                </ul>
                <p>Reassign or edit those connections before deleting.</p>
                <div class="actions">
                    <button class="btn" on:click={() => (deleting = null)}>OK</button>
                </div>
            {:else}
                <p>This action cannot be undone.</p>
                <div class="actions">
                    <button class="btn" on:click={() => (deleting = null)}>Cancel</button>
                    <button class="btn btn-danger" on:click={confirmDelete}>Delete</button>
                </div>
            {/if}
        </div>
    </div>
{/if}

<style>
    h2 { margin: 0; font-size: 14px; font-weight: 600; color: var(--ink); }
    h3 { margin: 0 0 8px; font-size: 13px; font-weight: 600; color: var(--ink); }

    .header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }

    .err {
        padding: 8px 10px;
        background: var(--danger-soft);
        color: var(--danger);
        border-radius: var(--r-sm);
        margin-bottom: 8px;
        font-size: 12.5px;
    }

    .loading { color: var(--muted); font-size: 12.5px; padding: 8px 0; }

    .empty { padding: 24px; text-align: center; color: var(--muted); font-size: 12.5px; }

    .btn-link {
        background: none;
        border: none;
        color: var(--accent);
        cursor: pointer;
        font: inherit;
        font-size: 12.5px;
        padding: 0;
        text-decoration: underline;
    }
    .btn-link:hover { color: var(--accent-hover); }

    ul.profiles { list-style: none; padding: 0; margin: 0; }
    ul.profiles li { display: flex; align-items: center; gap: 8px; padding: 3px 0; }

    button.row {
        flex: 1;
        text-align: left;
        padding: 7px 10px;
        background: var(--bg-elevated);
        border: 1px solid var(--border);
        border-radius: var(--r-sm);
        cursor: pointer;
        display: flex;
        justify-content: space-between;
        align-items: center;
        gap: 12px;
        font: inherit;
        color: inherit;
        transition: background var(--t-fast) var(--ease-out);
    }
    button.row:hover { background: var(--bg-hover); }

    button.delete {
        padding: 4px 8px;
        border: 1px solid var(--border);
        background: var(--bg-elevated);
        border-radius: var(--r-sm);
        cursor: pointer;
        font: inherit;
        color: var(--muted);
        transition: background var(--t-fast) var(--ease-out), color var(--t-fast) var(--ease-out);
    }
    button.delete:hover { background: var(--danger-soft); color: var(--danger); border-color: var(--danger); }

    .name { font-weight: 500; font-size: 12.5px; color: var(--ink); }
    .target { color: var(--muted); font-family: var(--font-mono); font-size: 11.5px; flex-shrink: 0; }

    .field-row {
        display: flex;
        align-items: center;
        gap: 8px;
        margin: 6px 0;
    }
    .field-row .lbl {
        width: 60px;
        flex-shrink: 0;
        font-size: 12px;
        color: var(--muted);
        text-align: right;
    }
    .field-row .field { flex: 1; }
    .field-row .field-short { width: 80px; flex: none; }
    .field-row .btn-browse { flex-shrink: 0; }

    .field {
        height: 28px;
        padding: 0 8px;
        background: var(--bg-content);
        color: var(--ink);
        border: 1px solid var(--border-strong);
        border-radius: var(--r-sm);
        font: inherit;
        font-size: 12.5px;
        transition: border-color var(--t-fast) var(--ease-out), box-shadow var(--t-fast) var(--ease-out);
    }
    .field::placeholder { color: var(--faint); }
    .field:focus { outline: none; border-color: var(--accent); box-shadow: 0 0 0 3px var(--focus-ring); }

    fieldset {
        margin: 10px 0;
        padding: 8px 10px;
        border: 1px solid var(--border);
        border-radius: var(--r-sm);
        font-size: 12.5px;
    }
    fieldset legend { font-size: 11.5px; color: var(--muted); padding: 0 4px; }

    .radio-row {
        display: flex;
        align-items: center;
        gap: 6px;
        margin: 4px 0;
        cursor: pointer;
        color: var(--ink-soft);
        font-size: 12.5px;
    }
    .radio-row input[type="radio"] { accent-color: var(--accent); }

    .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px; }

    /* Use project's .btn pattern locally */
    .btn {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: 4px;
        height: 28px;
        padding: 0 12px;
        border-radius: var(--r-sm);
        font: inherit;
        font-size: 12.5px;
        font-weight: 500;
        white-space: nowrap;
        background: var(--bg-elevated);
        color: var(--ink-soft);
        border: 1px solid var(--border-strong);
        cursor: pointer;
        transition: background var(--t-fast) var(--ease-out), border-color var(--t-fast) var(--ease-out);
    }
    .btn:hover { background: var(--bg-hover); color: var(--ink); }
    .btn:active { background: var(--bg-active); }

    .btn.btn-primary {
        background: var(--accent);
        color: var(--accent-ink);
        border-color: transparent;
        font-weight: 600;
    }
    .btn.btn-primary:hover { background: var(--accent-hover); }
    .btn.btn-primary:active { background: var(--accent-press); }

    .btn.btn-danger {
        background: var(--danger);
        color: #fff;
        border-color: transparent;
        font-weight: 600;
    }
    .btn.btn-danger:hover { opacity: 0.88; }
    .btn.btn-danger:active { opacity: 0.75; }

    /* Modal / backdrop */
    .backdrop {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.5);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: var(--z-modal);
    }
    .modal {
        background: var(--bg-panel);
        color: var(--ink);
        border: 1px solid var(--border-strong);
        border-radius: var(--r-lg);
        padding: 20px 24px;
        max-width: 480px;
        width: 90vw;
        box-shadow: var(--shadow-modal);
        font-size: 12.5px;
    }
    .modal p { margin: 8px 0; color: var(--ink-soft); }
    .dep-list { margin: 8px 0; padding-left: 20px; color: var(--ink); }
    .dep-list li { margin: 3px 0; }
</style>
