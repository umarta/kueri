<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import Modal from "./Modal.svelte";

  export let value: unknown = null;
  export let columnType = "";
  export let columnName = "";

  type Mode = "auto" | "json" | "text" | "image" | "hex";

  const dispatch = createEventDispatcher<{ close: void }>();

  // ── Detection ────────────────────────────────────────────────────────────
  const raw = value == null ? "" : String(value);
  const type = columnType.toLowerCase();

  function looksLikeJson(s: string): boolean {
    const t = s.trim();
    if (!(t.startsWith("{") || t.startsWith("["))) return false;
    try { JSON.parse(t); return true; } catch { return false; }
  }

  function looksLikeBinary(t: string): boolean {
    return /^(bytea|blob|bytes|binary|varbinary|image)$/i.test(t) || t.startsWith("bit");
  }

  // Postgres bytea comes back as "\\xdeadbeef…"; MySQL/SQLite as raw bytes in a string.
  function decodeBytes(s: string): Uint8Array | null {
    if (!s) return null;
    if (s.startsWith("\\x") || s.startsWith("\\X")) {
      const hex = s.slice(2).replace(/[^0-9a-fA-F]/g, "");
      if (hex.length % 2 !== 0) return null;
      const out = new Uint8Array(hex.length / 2);
      for (let i = 0; i < hex.length; i += 2) out[i / 2] = parseInt(hex.slice(i, i + 2), 16);
      return out;
    }
    // Fallback: treat as-is (each char code = byte). Loses high code points but
    // works for values sqlx returns as latin-1 strings.
    const out = new Uint8Array(s.length);
    for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i) & 0xff;
    return out;
  }

  function imageMime(bytes: Uint8Array): string | null {
    if (bytes.length >= 8 && bytes[0] === 0x89 && bytes[1] === 0x50 && bytes[2] === 0x4e && bytes[3] === 0x47) return "image/png";
    if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) return "image/jpeg";
    if (bytes.length >= 6 && bytes[0] === 0x47 && bytes[1] === 0x49 && bytes[2] === 0x46) return "image/gif";
    if (bytes.length >= 12 && bytes[0] === 0x52 && bytes[1] === 0x49 && bytes[2] === 0x46 && bytes[3] === 0x46 && bytes[8] === 0x57 && bytes[9] === 0x45 && bytes[10] === 0x42 && bytes[11] === 0x50) return "image/webp";
    return null;
  }

  function detectMode(): Mode {
    if (/(^|\W)(json|jsonb)$/i.test(type) || looksLikeJson(raw)) return "json";
    if (looksLikeBinary(type)) {
      const b = decodeBytes(raw);
      if (b && imageMime(b)) return "image";
      return "hex";
    }
    return "text";
  }

  let mode: Mode = detectMode();

  // ── Data derived from mode ───────────────────────────────────────────────
  $: bytes = mode === "hex" || mode === "image" ? decodeBytes(raw) : null;
  $: mime = mode === "image" && bytes ? imageMime(bytes) : null;
  $: imgSrc = mode === "image" && bytes && mime ? bytesToDataUrl(bytes, mime) : "";
  $: parsedJson = mode === "json" ? tryParseJson(raw) : null;
  $: jsonItems = isObj(parsedJson)
    ? Object.entries(parsedJson)
    : Array.isArray(parsedJson)
      ? (parsedJson as unknown[]).map((x, i) => [String(i), x] as [string, unknown])
      : [];

  function bytesToDataUrl(b: Uint8Array, m: string): string {
    let bin = "";
    for (let i = 0; i < b.length; i++) bin += String.fromCharCode(b[i]);
    return `data:${m};base64,${btoa(bin)}`;
  }

  function tryParseJson(s: string): unknown {
    try { return JSON.parse(s); } catch { return null; }
  }

  // ── Copy ─────────────────────────────────────────────────────────────────
  let copied = false;
  async function copy() {
    try {
      await navigator.clipboard.writeText(raw);
      copied = true;
      setTimeout(() => (copied = false), 1200);
    } catch { /* ignore */ }
  }

  // ── JSON tree ────────────────────────────────────────────────────────────
  let jsonFilter = "";
  $: jsonNorm = (jsonFilter ?? "").trim().toLowerCase();

  function isObj(v: unknown): v is Record<string, unknown> {
    return typeof v === "object" && v !== null && !Array.isArray(v);
  }

  function matches(v: unknown, needle: string, path = ""): boolean {
    if (!needle) return true;
    if (path.toLowerCase().includes(needle)) return true;
    if (typeof v === "string" || typeof v === "number" || typeof v === "boolean") {
      return String(v).toLowerCase().includes(needle);
    }
    if (Array.isArray(v)) return v.some((x, i) => matches(x, needle, `${path}[${i}]`));
    if (isObj(v)) return Object.entries(v).some(([k, x]) => matches(x, needle, path ? `${path}.${k}` : k));
    return false;
  }

  // ── Hex dump ─────────────────────────────────────────────────────────────
  const HEX_MAX = 65536; // don't render more than 64 KB inline

  function hexLines(b: Uint8Array): { offset: string; hex: string; ascii: string }[] {
    const n = Math.min(b.length, HEX_MAX);
    const out: { offset: string; hex: string; ascii: string }[] = [];
    for (let i = 0; i < n; i += 16) {
      const chunk = b.slice(i, i + 16);
      const hex = Array.from(chunk).map((v) => v.toString(16).padStart(2, "0")).join(" ");
      const ascii = Array.from(chunk).map((v) => (v >= 0x20 && v < 0x7f ? String.fromCharCode(v) : ".")).join("");
      out.push({ offset: i.toString(16).padStart(8, "0"), hex, ascii });
    }
    return out;
  }

  $: hex = mode === "hex" && bytes ? hexLines(bytes) : [];
  $: truncated = bytes && bytes.length > HEX_MAX;

  const TEXT_MAX = 1_000_000; // 1 MB soft cap
  $: textTruncated = (mode === "text" || mode === "json") && raw.length > TEXT_MAX;
  $: displayText = textTruncated ? raw.slice(0, TEXT_MAX) : raw;
</script>

<Modal title={columnName ? `Cell — ${columnName}` : "Cell"} width="720px" on:close={() => dispatch("close")}>
  <div class="toolbar">
    <label class="mode">
      Mode
      <select bind:value={mode}>
        <option value="text">Text</option>
        <option value="json">JSON tree</option>
        <option value="hex">Hex dump</option>
        <option value="image">Image</option>
      </select>
    </label>
    <span class="meta">
      {#if columnType}<code>{columnType}</code>{/if}
      {#if bytes}<span>· {bytes.length} bytes</span>{:else}<span>· {raw.length} chars</span>{/if}
      {#if truncated}<span class="warn">· hex truncated to {HEX_MAX} bytes</span>{/if}
      {#if textTruncated}<span class="warn">· text truncated to {TEXT_MAX / 1000} KB</span>{/if}
    </span>
    <button class="copy" on:click={copy}>{copied ? "Copied" : "Copy raw"}</button>
  </div>

  <div class="viewer">
    {#if mode === "json"}
      {#if parsedJson === null && raw.trim() !== ""}
        <p class="err">Value could not be parsed as JSON.</p>
      {:else}
        <input class="filter" type="text" placeholder="Filter keys or values…" bind:value={jsonFilter} />
        <div class="tree" role="tree">
          {#if jsonItems.length === 0}
            <pre class="leaf-pre">{JSON.stringify(parsedJson, null, 2)}</pre>
          {:else}
            {#each jsonItems as [k, child] (k)}
              {#if matches(child, jsonNorm, k)}
                <details open>
                  <summary><span class="k">{k}</span>: <span class="t">{Array.isArray(child) ? `Array(${child.length})` : isObj(child) ? "Object" : typeof child}</span></summary>
                  <pre class="leaf-pre">{JSON.stringify(child, null, 2)}</pre>
                </details>
              {/if}
            {/each}
          {/if}
        </div>
      {/if}
    {:else if mode === "image"}
      {#if bytes && mime}
        <div class="img-wrap"><img src={imgSrc} alt={columnName} /></div>
      {:else}
        <p class="err">No image signature detected in this value.</p>
      {/if}
    {:else if mode === "hex"}
      {#if bytes && bytes.length > 0}
        <pre class="hex">{#each hex as line (line.offset)}<div class="hex-row"><span class="off">{line.offset}</span>  {line.hex.padEnd(48, " ")}  <span class="asc">{line.ascii}</span></div>{/each}</pre>
      {:else}
        <p class="err">No bytes to display.</p>
      {/if}
    {:else}
      <pre class="text">{displayText || ""}</pre>
    {/if}
  </div>
</Modal>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--s-4);
    margin-bottom: var(--s-4);
    font-size: 12px;
  }
  .mode {
    display: inline-flex;
    align-items: center;
    gap: var(--s-2);
    color: var(--muted);
  }
  .mode select {
    background: var(--bg-content);
    border: 1px solid var(--border);
    border-radius: var(--r-xs);
    padding: 2px 6px;
    font-size: 12px;
    color: var(--ink);
  }
  .meta {
    flex: 1;
    color: var(--muted);
    display: inline-flex;
    align-items: center;
    gap: var(--s-2);
    font-size: 11px;
  }
  .meta code {
    font-family: var(--font-mono);
    padding: 1px 4px;
    background: var(--bg-elevated);
    border-radius: var(--r-xs);
  }
  .warn { color: var(--warn); }
  .copy {
    padding: 4px 10px;
    background: var(--bg-content);
    border: 1px solid var(--border);
    border-radius: var(--r-xs);
    font-size: 11px;
    color: var(--ink);
  }
  .copy:hover { background: var(--bg-elevated); }

  .viewer {
    max-height: 60vh;
    overflow: auto;
    font-family: var(--font-mono);
    font-size: 12px;
    background: var(--bg-content);
    border: 1px solid var(--hairline);
    border-radius: var(--r-sm);
    padding: var(--s-3);
  }
  .text {
    white-space: pre-wrap;
    word-break: break-word;
    margin: 0;
    color: var(--ink);
  }
  .err { color: var(--muted); margin: 0; }

  .filter {
    width: 100%;
    padding: 4px 8px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--r-xs);
    color: var(--ink);
    margin-bottom: var(--s-2);
    font-family: inherit;
    font-size: 12px;
  }
  .tree details { margin: 2px 0; }
  .tree summary { cursor: pointer; color: var(--ink-soft); }
  .tree .k { color: var(--accent); }
  .tree .t { color: var(--muted); font-style: italic; }
  .leaf-pre {
    margin: 4px 0 4px 16px;
    padding: 6px 8px;
    background: var(--bg-panel);
    border-radius: var(--r-xs);
    color: var(--ink);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .hex { margin: 0; }
  .hex-row { display: flex; gap: var(--s-3); white-space: pre; }
  .off { color: var(--muted); }
  .asc { color: var(--ink-soft); }

  .img-wrap {
    display: grid;
    place-items: center;
    padding: var(--s-3);
    background: repeating-conic-gradient(rgba(255,255,255,0.03) 0% 25%, transparent 0% 50%) 50% / 16px 16px;
    border-radius: var(--r-xs);
  }
  .img-wrap img { max-width: 100%; max-height: 55vh; }
</style>
