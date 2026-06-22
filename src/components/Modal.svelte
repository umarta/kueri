<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";

  export let title = "";
  export let width = "440px";

  const dispatch = createEventDispatcher<{ close: void }>();
  let card: HTMLDivElement;

  function close() {
    dispatch("close");
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    }
  }

  onMount(() => {
    // Focus the first focusable element so the dialog is keyboard-ready.
    const first = card.querySelector<HTMLElement>(
      "input, button, select, [tabindex]:not([tabindex='-1'])"
    );
    first?.focus();
  });
</script>

<svelte:window on:keydown={onKey} />

<div
  class="backdrop"
  role="button"
  tabindex="-1"
  on:click|self={close}
  on:keydown={() => {}}
>
  <div
    class="card"
    bind:this={card}
    role="dialog"
    aria-modal="true"
    aria-label={title}
    style="--w: {width}"
  >
    {#if title}
      <header class="head"><h2>{title}</h2></header>
    {/if}
    <div class="body"><slot /></div>
    {#if $$slots.footer}
      <footer class="foot"><slot name="footer" /></footer>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: var(--z-modal);
    display: grid;
    place-items: center;
    padding: 40px;
    background: rgba(0, 0, 0, 0.45);
    animation: fade var(--t-base) var(--ease-out);
  }
  .card {
    width: min(var(--w), 100%);
    max-height: calc(100vh - 80px);
    display: flex;
    flex-direction: column;
    background: var(--bg-panel);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-lg);
    box-shadow: var(--shadow-modal);
    animation: rise var(--t-base) var(--ease-out);
    overflow: hidden;
  }
  .head {
    padding: var(--s-6) var(--s-7) 0;
    text-align: center;
  }
  .head h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    letter-spacing: -0.01em;
    color: var(--ink);
  }
  .body {
    padding: var(--s-6) var(--s-7);
    overflow: auto;
  }
  .foot {
    display: flex;
    gap: var(--s-3);
    justify-content: flex-end;
    align-items: center;
    padding: var(--s-5) var(--s-7);
    border-top: 1px solid var(--hairline);
    background: rgba(0, 0, 0, 0.12);
  }
  @keyframes fade {
    from { opacity: 0; }
  }
  @keyframes rise {
    from { opacity: 0; transform: translateY(8px) scale(0.985); }
  }
  @media (prefers-reduced-motion: reduce) {
    .backdrop, .card { animation: none; }
  }
</style>
