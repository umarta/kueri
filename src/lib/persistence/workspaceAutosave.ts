// Cross-launch workspace snapshot autosave (KUE-006).
// Subscribes to workspaceStates + activeConnectionId, debounces writes to
// workspaces.json via api.saveWorkspaces. Load-on-boot lives in App.svelte;
// callers must invoke markHydrated() after hydration completes to arm autosave.

import { get } from "svelte/store";
import { api } from "../tauri";
import type {
  PersistedTab,
  PersistedWorkspace,
  WorkspaceFile,
} from "../types";
import { workspaceStates } from "../stores/workspaces";
import { activeConnectionId } from "../stores/connection";

let hydrated = false;
let scheduledTimer: number | null = null;

/** Project every live workspace down to its persistable skeleton. */
export function serializeAll(): WorkspaceFile {
  const states = get(workspaceStates);
  const activeId = get(activeConnectionId);
  const workspaces: PersistedWorkspace[] = [];
  for (const [connId, w] of states.entries()) {
    const tabs: PersistedTab[] = [];
    for (const t of w.tabs) {
      if (t.kind === "query") {
        tabs.push({ kind: "query", id: t.id, title: t.title, sql: t.doc ?? "" });
      } else if (t.kind === "table" && t.selected) {
        tabs.push({
          kind: "table",
          id: t.id,
          schema: t.selected.schema,
          table: t.selected.table,
        });
      }
      // Any other tab shape (or a table tab without a selection) is dropped.
    }
    workspaces.push({
      connection_id: connId,
      active_schema: w.activeSchema ?? "",
      focused_tab_id: w.focusedTabId ?? null,
      tabs,
    });
  }
  return {
    schema_version: 1,
    last_active_id: activeId ?? null,
    workspaces,
  };
}

/** Debounced save. No-op while !hydrated (guards the initial subscription tick). */
export function scheduleSave(delayMs = 500): void {
  if (!hydrated) return;
  if (scheduledTimer !== null) window.clearTimeout(scheduledTimer);
  scheduledTimer = window.setTimeout(async () => {
    scheduledTimer = null;
    try {
      await api.saveWorkspaces(serializeAll());
    } catch (e) {
      // Never throw from autosave. Boot must not depend on write success.
      console.warn("workspace autosave failed:", e);
    }
  }, delayMs);
}

/** Flip the guard after boot-load completes. */
export function markHydrated(): void {
  hydrated = true;
}

/** Attach store subscriptions. Returns an unsubscribe function (rarely needed). */
export function startAutosave(): () => void {
  const off1 = workspaceStates.subscribe(() => scheduleSave());
  const off2 = activeConnectionId.subscribe(() => scheduleSave());
  return () => {
    off1();
    off2();
  };
}
