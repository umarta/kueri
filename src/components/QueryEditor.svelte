<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from "svelte";
  import { EditorState, Compartment } from "@codemirror/state";
  import {
    EditorView, keymap, lineNumbers, highlightActiveLine,
    highlightActiveLineGutter, drawSelection, dropCursor,
  } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { bracketMatching, indentOnInput } from "@codemirror/language";
  import { autocompletion, completionKeymap, closeBrackets, closeBracketsKeymap } from "@codemirror/autocomplete";
  import { search, searchKeymap } from "@codemirror/search";
  import { sql, schemaCompletionSource, keywordCompletionSource, PostgreSQL, MySQL, SQLite, MSSQL, StandardSQL, type SQLDialect } from "@codemirror/lang-sql";
  import { format as formatSql } from "sql-formatter";
  import { kueriEditorTheme } from "../lib/editor/theme";
  import { paramExtension, paramField } from "../lib/editor/params";
  import { makeColumnCompletions } from "../lib/editor/completions";
  import { splitStatements } from "../lib/sql/split";
  import { substituteParams } from "../lib/sql/params";
  import type { DbKind } from "../lib/types";

  export let running = false;
  export let dialect: DbKind = "postgres";
  export let schema: Record<string, string[]> = {};
  export let initialDoc = "SELECT now();";
  export let connectionId: string | null = null;
  export let activeSchema: string = "public";
  /** Named param values — bound two-way from parent via bind:params */
  export let params: Record<string, string> = {};

  const dispatch = createEventDispatcher<{ run: { sql: string; params: unknown[] }; change: string }>();

  let host: HTMLDivElement;
  let view: EditorView;
  const sqlConf = new Compartment();
  // Incremented on every doc change so docParams reactive re-evaluates.
  let docVersion = 0;

  const DIALECTS: Record<string, SQLDialect> = {
    postgres: PostgreSQL,
    mysql: MySQL,
    sqlite: SQLite,
    sqlserver: MSSQL,
  };

  function sqlExtension() {
    const cfg = {
      dialect: DIALECTS[dialect] ?? StandardSQL,
      schema,
      upperCaseKeywords: true,
      defaultSchema: activeSchema,
    };
    return [
      sql(cfg),
      autocompletion({
        activateOnTyping: true,
        override: [
          schemaCompletionSource(cfg),
          keywordCompletionSource(cfg.dialect ?? StandardSQL, true),
          makeColumnCompletions(() => connectionId, () => activeSchema),
        ],
      }),
    ];
  }

  // Cursor-aware run: selection → selected text; else → statement at cursor; else → whole doc.
  function runCurrent(v: EditorView): boolean {
    const { state } = v;
    const sel = state.selection.main;
    let text: string;
    if (!sel.empty) {
      text = state.sliceDoc(sel.from, sel.to);
    } else {
      const doc = state.doc.toString();
      const cursor = sel.head;
      const stmts = splitStatements(doc);
      const hit = stmts.find((s) => s.from <= cursor && cursor <= s.to);
      text = hit ? hit.text : doc;
    }
    if (!text.trim()) return true;
    dispatchRun(text);
    return true;
  }

  function runAll(v: EditorView): boolean {
    const doc = v.state.doc.toString();
    if (!doc.trim()) return true;
    dispatchRun(doc);
    return true;
  }

  function dispatchRun(rawSql: string) {
    const dialectKey = (dialect === "postgres" ? "postgres" : dialect === "mysql" ? "mysql" : "sqlite") as "postgres" | "mysql" | "sqlite";
    const { sql: substituted, ordered } = substituteParams(rawSql, dialectKey, []);
    const boundValues = ordered.map((name) => params[name] ?? "");
    dispatch("run", { sql: substituted, params: boundValues });
  }

  /** Format the buffer with sql-formatter (⌘⇧F). */
  export function format() {
    if (!view) return;
    const lang = dialect === "mysql" ? "mysql" : dialect === "sqlite" ? "sqlite" : "postgresql";
    let out: string;
    try {
      out = formatSql(view.state.doc.toString(), { language: lang, keywordCase: "upper" });
    } catch {
      return;
    }
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: out } });
    dispatch("change", out);
  }

  function formatCmd(_v: EditorView): boolean {
    format();
    return true;
  }

  /** Replace the editor content (loading a saved/history query). */
  export function setDoc(text: string) {
    if (!view) return;
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } });
    dispatch("change", text);
  }

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: initialDoc,
        extensions: [
          lineNumbers(),
          highlightActiveLine(),
          highlightActiveLineGutter(),
          history(),
          drawSelection(),
          dropCursor(),
          indentOnInput(),
          bracketMatching(),
          closeBrackets(),
          search({ top: true }),
          ...paramExtension,
          keymap.of([
            { key: "Mod-Enter", preventDefault: true, run: runCurrent },
            { key: "Mod-Shift-Enter", preventDefault: true, run: runAll },
            { key: "Mod-Shift-f", preventDefault: true, run: formatCmd },
            ...closeBracketsKeymap,
            ...searchKeymap,
            ...defaultKeymap,
            ...historyKeymap,
            ...completionKeymap,
            indentWithTab,
          ]),
          sqlConf.of(sqlExtension()),
          EditorView.updateListener.of((u) => {
            if (u.docChanged) { docVersion += 1; dispatch("change", u.state.doc.toString()); }
          }),
          kueriEditorTheme,
          EditorView.lineWrapping,
        ],
      }),
    });
  });

  onDestroy(() => view?.destroy());

  $: if (view) view.dispatch({ effects: sqlConf.reconfigure(sqlExtension()) });

  // Derive unique params in appearance order for the input row.
  // docVersion dependency ensures this re-runs on every doc edit.
  $: docParams = view && docVersion >= 0
    ? (() => {
        const refs = view.state.field(paramField, false) ?? [];
        const seen = new Set<string>();
        return refs.filter((r) => { if (seen.has(r.name)) return false; seen.add(r.name); return true; });
      })()
    : [];

  function runClick() {
    if (view) runCurrent(view);
  }
</script>

<div class="editor">
  <div class="cm-host" bind:this={host}></div>
  {#if docParams.length > 0}
    <div class="params-row">
      {#each docParams as ref (ref.name)}
        <label class="param-label">
          <span class="param-name">{ref.name}</span>
          <input
            class="param-input"
            type="text"
            placeholder="value"
            bind:value={params[ref.name]}
          />
        </label>
      {/each}
    </div>
  {/if}
  <div class="bar">
    <span class="hint">⌘↵ run · ⌘⇧↵ run all · ⌘⇧F format</span>
    <button class="btn btn-primary" on:click={runClick} disabled={running}>
      {#if running}<span class="spin" aria-hidden="true"></span> Running…{:else}Run{/if}
    </button>
  </div>
</div>

<style>
  .editor { display: flex; flex-direction: column; border-bottom: 1px solid var(--border); background: var(--bg-content); }
  .cm-host { height: 140px; min-height: 64px; overflow: hidden; resize: vertical; }
  :global(.cm-host .cm-editor) { height: 100%; }
  :global(.cm-param) {
    background-color: rgba(251, 191, 36, 0.15);
    border-radius: 2px;
    outline: 1px solid rgba(251, 191, 36, 0.4);
  }
  .params-row {
    display: flex; flex-wrap: wrap; gap: var(--s-2) var(--s-4);
    padding: var(--s-2) var(--s-5);
    background: var(--bg-panel); border-top: 1px solid var(--hairline);
    font-size: 12px;
  }
  .param-label { display: inline-flex; align-items: center; gap: var(--s-1); }
  .param-name { color: var(--accent); font-family: var(--font-mono); font-size: 11px; }
  .param-input {
    width: 120px; padding: 2px 6px;
    background: var(--bg-content); border: 1px solid var(--border);
    border-radius: var(--r-xs); color: var(--ink); font-family: var(--font-mono); font-size: 11px;
  }
  .bar {
    display: flex; align-items: center; justify-content: flex-end; gap: var(--s-4);
    padding: var(--s-3) var(--s-5); background: var(--bg-panel); border-top: 1px solid var(--hairline);
  }
  .hint { font-size: 11px; color: var(--faint); font-family: var(--font-mono); }
  .spin {
    width: 12px; height: 12px; border-radius: 50%;
    border: 2px solid rgba(255,255,255,0.4); border-top-color: #fff;
    animation: spin 0.6s linear infinite; display: inline-block;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .spin { animation-duration: 1ms; } }
</style>
