import { StateField } from "@codemirror/state";
import { Decoration, EditorView, keymap } from "@codemirror/view";
import type { Extension } from "@codemirror/state";
import { parseParams, type ParamRef } from "../sql/params";

export const paramField = StateField.define<ParamRef[]>({
  create(state) {
    return parseParams(state.doc.toString());
  },
  update(refs, tr) {
    if (!tr.docChanged) return refs;
    return parseParams(tr.newDoc.toString());
  },
});

const paramMark = Decoration.mark({ class: "cm-param" });

const paramDecorations = EditorView.decorations.from(paramField, (refs) =>
  Decoration.set(
    refs.map((r) => paramMark.range(r.from, r.to)),
    true
  )
);

function jumpParam(view: EditorView, forward: boolean): boolean {
  const refs = view.state.field(paramField);
  if (refs.length === 0) return false;
  const cursor = view.state.selection.main.head;
  const inParam = refs.some((r) => cursor >= r.from && cursor <= r.to);
  if (!inParam) return false;
  let target: ParamRef | undefined;
  if (forward) {
    target = refs.find((r) => r.from > cursor) ?? refs[0];
  } else {
    target = [...refs].reverse().find((r) => r.to < cursor) ?? refs[refs.length - 1];
  }
  if (!target) return false;
  view.dispatch({ selection: { anchor: target.from + 1 } });
  return true;
}

const paramKeymap = keymap.of([
  { key: "Tab", run: (v) => jumpParam(v, true) },
  { key: "Shift-Tab", run: (v) => jumpParam(v, false) },
]);

export const paramExtension: Extension[] = [paramField, paramDecorations, paramKeymap];
