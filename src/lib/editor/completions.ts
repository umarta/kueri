import type { CompletionContext, CompletionResult } from "@codemirror/autocomplete";
import { syntaxTree } from "@codemirror/language";
import { api } from "../tauri";
import type { ColumnInfo } from "../types";

const colCache = new Map<string, ColumnInfo[]>();

export function clearCompletionCache(connectionId?: string) {
  if (connectionId) {
    for (const key of colCache.keys()) {
      if (key.startsWith(`${connectionId}:`)) colCache.delete(key);
    }
  } else {
    colCache.clear();
  }
}

export function makeColumnCompletions(
  getConnectionId: () => string | null,
  getSchema: () => string
) {
  return async function (ctx: CompletionContext): Promise<CompletionResult | null> {
    const connectionId = getConnectionId();
    if (!connectionId) return null;

    // Bail out when cursor is inside a string or comment.
    const node = syntaxTree(ctx.state).resolveInner(ctx.pos, -1);
    if (["String", "LineComment", "BlockComment"].includes(node.name)) return null;

    const word = ctx.matchBefore(/[\w.]+/);
    if (!word || (word.from === word.to && !ctx.explicit)) return null;

    const text = word.text;
    const dotIdx = text.lastIndexOf(".");
    if (dotIdx === -1) return null; // bare word — fall through to built-in keyword completion

    const tablePart = text.slice(0, dotIdx);
    const partial = text.slice(dotIdx + 1).toLowerCase();
    const schema = getSchema();
    const cacheKey = `${connectionId}:${schema}:${tablePart}`;

    let cols = colCache.get(cacheKey);
    if (!cols) {
      try {
        cols = await api.listColumns(connectionId, schema, tablePart);
        colCache.set(cacheKey, cols);
      } catch {
        return null;
      }
    }

    const options = cols
      .filter((c) => c.name.toLowerCase().startsWith(partial))
      .map((c) => ({ label: c.name, detail: c.data_type, type: "property" as const }));

    if (!options.length) return null;
    return { from: word.from + dotIdx + 1, options };
  };
}
