import { EditorView } from "@codemirror/view";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";

// Editor chrome — wired to the app design tokens so it always matches the theme.
const kueriTheme = EditorView.theme(
  {
    "&": {
      color: "var(--ink)",
      backgroundColor: "var(--bg-content)",
      fontSize: "13px",
      height: "100%",
    },
    ".cm-scroller": {
      fontFamily: "var(--font-mono)",
      lineHeight: "1.6",
      padding: "var(--s-3) 0",
    },
    ".cm-content": { padding: "0", caretColor: "var(--accent)" },
    ".cm-line": { padding: "0 var(--s-6)" },
    "&.cm-focused": { outline: "none" },
    ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--accent)" },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection": {
      backgroundColor: "var(--accent-soft)",
    },
    ".cm-gutters": {
      backgroundColor: "var(--bg-content)",
      color: "var(--faint)",
      border: "none",
      paddingRight: "var(--s-2)",
    },
    ".cm-activeLineGutter": { backgroundColor: "transparent", color: "var(--muted)" },
    ".cm-activeLine": { backgroundColor: "rgba(255,255,255,0.03)" },
    ".cm-selectionMatch": { backgroundColor: "rgba(10,132,255,0.16)" },
    ".cm-matchingBracket, &.cm-focused .cm-matchingBracket": {
      backgroundColor: "rgba(10,132,255,0.22)",
      outline: "1px solid var(--accent)",
    },
    // Autocomplete popup
    ".cm-tooltip": {
      backgroundColor: "var(--bg-elevated)",
      border: "1px solid var(--border-strong)",
      borderRadius: "var(--r-md)",
      boxShadow: "var(--shadow-pop)",
      overflow: "hidden",
    },
    ".cm-tooltip-autocomplete > ul": { fontFamily: "var(--font-mono)", fontSize: "12px", maxHeight: "16em" },
    ".cm-tooltip-autocomplete > ul > li": { padding: "3px 10px", color: "var(--ink-soft)" },
    ".cm-tooltip-autocomplete > ul > li[aria-selected]": {
      backgroundColor: "var(--accent)",
      color: "var(--accent-ink)",
    },
    ".cm-completionIcon": { paddingRight: "14px", opacity: "0.6" },
    ".cm-completionLabel": { color: "inherit" },
    ".cm-completionDetail": { color: "var(--faint)", fontStyle: "normal", marginLeft: "auto", paddingLeft: "12px" },
  },
  { dark: true }
);

// Syntax colors — restrained: keywords/types carry meaning, strings/numbers warm.
const kueriHighlight = HighlightStyle.define([
  { tag: [t.keyword, t.operatorKeyword, t.modifier], color: "#c792ea", fontWeight: "600" },
  { tag: [t.string, t.special(t.string)], color: "#a5d6a3" },
  { tag: [t.number, t.bool, t.null], color: "#f5a25d" },
  { tag: [t.function(t.variableName), t.function(t.propertyName)], color: "#4db8ff" },
  { tag: [t.typeName, t.className], color: "#5cc8d8" },
  { tag: [t.propertyName, t.attributeName], color: "var(--ink)" },
  { tag: [t.variableName], color: "var(--ink-soft)" },
  { tag: [t.comment], color: "var(--faint)", fontStyle: "italic" },
  { tag: [t.operator, t.punctuation], color: "var(--muted)" },
  { tag: [t.bracket], color: "var(--muted)" },
]);

export const kueriEditorTheme = [kueriTheme, syntaxHighlighting(kueriHighlight)];
