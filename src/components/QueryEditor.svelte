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
  import { sql, PostgreSQL, MySQL, SQLite, MSSQL, StandardSQL, type SQLDialect } from "@codemirror/lang-sql";
  import { format as formatSql } from "sql-formatter";
  import { kueriEditorTheme } from "../lib/editor/theme";
  import type { DbKind } from "../lib/types";

  export let running = false;
  export let dialect: DbKind = "postgres";
  export let schema: Record<string, string[]> = {};
  export let initialDoc = "SELECT now();";

  const dispatch = createEventDispatcher<{ run: string; change: string }>();

  let host: HTMLDivElement;
  let view: EditorView;
  const sqlConf = new Compartment();

  const DIALECTS: Record<string, SQLDialect> = {
    postgres: PostgreSQL,
    mysql: MySQL,
    sqlite: SQLite,
    sqlserver: MSSQL,
  };

  function sqlExtension() {
    return sql({
      dialect: DIALECTS[dialect] ?? StandardSQL,
      schema,
      upperCaseKeywords: true,
    });
  }

  // Run selection if there is one, else the whole document — TablePlus behavior.
  function runQuery(v: EditorView): boolean {
    const { state } = v;
    const sel = state.selection.main;
    const text = sel.empty ? state.doc.toString() : state.sliceDoc(sel.from, sel.to);
    if (text.trim()) dispatch("run", text);
    return true;
  }

  /** Format the buffer with sql-formatter (Edit → Format SQL). */
  export function format() {
    if (!view) return;
    const lang = dialect === "mysql" ? "mysql" : dialect === "sqlite" ? "sqlite" : "postgresql";
    let out: string;
    try {
      out = formatSql(view.state.doc.toString(), { language: lang, keywordCase: "upper" });
    } catch {
      return; // leave the buffer untouched on parse error
    }
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: out } });
    dispatch("change", out);
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
          autocompletion({ activateOnTyping: true }),
          search({ top: true }),
          keymap.of([
            { key: "Mod-Enter", preventDefault: true, run: runQuery },
            ...closeBracketsKeymap,
            ...searchKeymap,
            ...defaultKeymap,
            ...historyKeymap,
            ...completionKeymap,
            indentWithTab,
          ]),
          sqlConf.of(sqlExtension()),
          EditorView.updateListener.of((u) => {
            if (u.docChanged) dispatch("change", u.state.doc.toString());
          }),
          kueriEditorTheme,
          EditorView.lineWrapping,
        ],
      }),
    });
  });

  onDestroy(() => view?.destroy());

  // Reconfigure SQL language when dialect or schema catalog changes.
  $: if (view) view.dispatch({ effects: sqlConf.reconfigure(sqlExtension()) });

  function runClick() {
    if (view) runQuery(view);
  }
</script>

<div class="editor">
  <div class="cm-host" bind:this={host}></div>
  <div class="bar">
    <span class="hint">⌘↵ to run · selection or all</span>
    <button class="btn btn-primary" on:click={runClick} disabled={running}>
      {#if running}<span class="spin" aria-hidden="true"></span> Running…{:else}Run{/if}
    </button>
  </div>
</div>

<style>
  .editor { display: flex; flex-direction: column; border-bottom: 1px solid var(--border); background: var(--bg-content); }
  .cm-host { height: 140px; min-height: 64px; overflow: hidden; resize: vertical; }
  :global(.cm-host .cm-editor) { height: 100%; }
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
