export type EditAction = { key: string; prev: string | null | undefined };

export function pushEdit(
  history: EditAction[],
  key: string,
  edits: Record<string, string | null>
): EditAction[] {
  return [...history, { key, prev: key in edits ? edits[key] : undefined }];
}

export function applyUndo(
  history: EditAction[],
  edits: Record<string, string | null>
): { history: EditAction[]; edits: Record<string, string | null> } {
  if (!history.length) return { history, edits };
  const nextHistory = history.slice(0, -1);
  const action = history[history.length - 1];
  const nextEdits = { ...edits };
  if (action.prev === undefined) {
    delete nextEdits[action.key];
  } else {
    nextEdits[action.key] = action.prev;
  }
  return { history: nextHistory, edits: nextEdits };
}
