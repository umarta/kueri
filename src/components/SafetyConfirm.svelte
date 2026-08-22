<script lang="ts">
    import { createEventDispatcher, tick } from "svelte";
    import { reasonText } from "../lib/safety/labels";
    import type { ConfirmReason } from "../lib/safety/labels";

    export let open = false;
    export let statement = "";
    export let reason: ConfirmReason = "destructive-no-where";

    const dispatch = createEventDispatcher<{ cancel: void; confirm: void }>();

    let cancelBtn: HTMLButtonElement;

    // Focus Cancel when the modal opens
    $: if (open) tick().then(() => cancelBtn?.focus());

    function onKey(e: KeyboardEvent) {
        if (!open) return;
        if (e.key === "Escape") { e.preventDefault(); dispatch("cancel"); }
    }
</script>

<svelte:window on:keydown={onKey} />

{#if open}
    <div class="backdrop" role="dialog" aria-modal="true" aria-labelledby="safety-confirm-title">
        <div class="modal">
            <h2 id="safety-confirm-title">Confirm statement</h2>
            <p class="reason">{reasonText(reason)}</p>
            <pre class="sql">{statement}</pre>
            <div class="actions">
                <button type="button" bind:this={cancelBtn} on:click={() => dispatch("cancel")}>
                    Cancel
                </button>
                <button type="button" class="primary" on:click={() => dispatch("confirm")}>
                    Run
                </button>
            </div>
        </div>
    </div>
{/if}

<style>
    .backdrop {
        position: fixed; inset: 0; background: rgba(0,0,0,0.45);
        display: flex; align-items: center; justify-content: center;
        z-index: var(--z-modal, 1000);
        animation: fade var(--t-base, 120ms) var(--ease-out, ease-out);
    }
    .modal {
        background: var(--bg-panel, #fff);
        color: var(--ink, #111);
        border: 1px solid var(--border-strong, #d1d5db);
        border-radius: var(--r-lg, 8px);
        padding: 20px 24px;
        max-width: 560px;
        width: 90vw;
        box-shadow: var(--shadow-modal, 0 12px 32px rgba(0,0,0,0.35));
        animation: rise var(--t-base, 120ms) var(--ease-out, ease-out);
    }
    .modal h2 { margin: 0 0 8px; font-size: 15px; font-weight: 600; }
    .reason { margin: 0 0 12px; color: var(--muted, #666); font-size: 13px; }
    .sql {
        font-family: var(--mono, ui-monospace, monospace);
        font-size: 12px;
        background: var(--bg-elevated, #f3f4f6);
        padding: 10px 12px;
        border-radius: var(--r-sm, 4px);
        overflow-x: auto;
        margin: 0 0 16px;
        white-space: pre-wrap;
        word-break: break-all;
    }
    .actions { display: flex; justify-content: flex-end; gap: 8px; }
    .actions button {
        padding: 6px 14px; border-radius: var(--r-sm, 4px);
        border: 1px solid var(--border, #d1d5db);
        background: var(--bg-panel, #fff); color: inherit; cursor: pointer;
        font-size: 13px;
    }
    .actions button:hover { background: var(--bg-elevated, #f3f4f6); }
    .actions button.primary {
        background: var(--danger, #dc2626);
        border-color: var(--danger, #dc2626);
        color: white;
    }
    .actions button.primary:hover { background: var(--danger-hover, #b91c1c); border-color: var(--danger-hover, #b91c1c); }
    @keyframes fade { from { opacity: 0; } }
    @keyframes rise { from { opacity: 0; transform: translateY(8px) scale(0.985); } }
    @media (prefers-reduced-motion: reduce) {
        .backdrop, .modal { animation: none; }
    }
</style>
