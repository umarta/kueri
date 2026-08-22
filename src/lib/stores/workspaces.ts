import { writable, derived, get, type Readable } from "svelte/store";
import type { QueryTab, SafetyLevel } from "../types";
import { activeConnectionId } from "./connection";

export type WorkspaceState = {
    connectionId: string;
    activeSchema: string;
    expandedNodes: Set<string>;
    schemaCatalog: Record<string, string[]>;
    tabs: QueryTab[];
    focusedTabId: string | null;
    inTransaction: boolean;
    safety: SafetyLevel;
    dismissedSafetyBanner: boolean;
};

function blankWorkspace(connectionId: string, safety: SafetyLevel = "confirm-destructive"): WorkspaceState {
    return {
        connectionId,
        activeSchema: "",
        expandedNodes: new Set<string>(),
        schemaCatalog: {},
        tabs: [],
        focusedTabId: null,
        inTransaction: false,
        safety,
        dismissedSafetyBanner: false,
    };
}

export const workspaceStates = writable<Map<string, WorkspaceState>>(new Map());

/** The workspace slice for the currently-active connection. */
export const currentWorkspace: Readable<WorkspaceState | null> = derived(
    [workspaceStates, activeConnectionId],
    ([states, id]) => (id ? states.get(id) ?? null : null),
);

// ── Derived re-exports that preserve the old global names ────────────────────
export const activeSchema: Readable<string> = derived(currentWorkspace, (w) => w?.activeSchema ?? "");
export const schemaCatalog: Readable<Record<string, string[]>> = derived(currentWorkspace, (w) => w?.schemaCatalog ?? {});
export const readOnly: Readable<boolean> = derived(currentWorkspace, (w) => w?.safety === "read-only");
export const inTransaction: Readable<boolean> = derived(currentWorkspace, (w) => w?.inTransaction ?? false);

// ── Lifecycle helpers ────────────────────────────────────────────────────────
export function ensureWorkspace(connId: string, defaultSafety?: SafetyLevel): void {
    workspaceStates.update((states) => {
        if (!states.has(connId)) {
            const next = new Map(states);
            next.set(connId, blankWorkspace(connId, defaultSafety));
            return next;
        }
        return states;
    });
}

export function dropWorkspace(connId: string): void {
    workspaceStates.update((states) => {
        if (!states.has(connId)) return states;
        const next = new Map(states);
        next.delete(connId);
        return next;
    });
}

// ── Mutators (all key on connId) ─────────────────────────────────────────────
function mutate(connId: string, fn: (w: WorkspaceState) => void): void {
    workspaceStates.update((states) => {
        const ws = states.get(connId);
        if (!ws) return states;
        fn(ws);
        return new Map(states); // trigger subscribers
    });
}

export function setActiveSchema(connId: string, schema: string): void {
    mutate(connId, (w) => { w.activeSchema = schema; });
}

export function setSafety(connId: string, level: SafetyLevel): void {
    mutate(connId, (w) => { w.safety = level; });
}


export function setInTransaction(connId: string, on: boolean): void {
    mutate(connId, (w) => { w.inTransaction = on; });
}

export function dismissBanner(connId: string): void {
    mutate(connId, (w) => { w.dismissedSafetyBanner = true; });
}

export function catalogTables(connId: string, tables: string[]): void {
    mutate(connId, (w) => {
        for (const t of tables) if (!w.schemaCatalog[t]) w.schemaCatalog[t] = [];
    });
}

export function catalogColumns(connId: string, table: string, columns: string[]): void {
    if (!table || columns.length === 0) return;
    mutate(connId, (w) => { w.schemaCatalog[table] = columns; });
}

// ── Tab helpers ──────────────────────────────────────────────────────────────
export function addTab(connId: string, tab: QueryTab): void {
    mutate(connId, (w) => {
        w.tabs.push(tab);
        w.focusedTabId = tab.id;
    });
}

export function removeTab(connId: string, tabId: string): void {
    mutate(connId, (w) => {
        const idx = w.tabs.findIndex((t) => t.id === tabId);
        if (idx === -1) return;
        w.tabs.splice(idx, 1);
        if (w.focusedTabId === tabId) {
            w.focusedTabId = w.tabs[Math.min(idx, w.tabs.length - 1)]?.id ?? null;
        }
    });
}

export function updateTab(connId: string, tabId: string, fn: (t: QueryTab) => void): void {
    mutate(connId, (w) => {
        const t = w.tabs.find((x) => x.id === tabId);
        if (t) fn(t);
    });
}

export function focusTab(connId: string, tabId: string): void {
    mutate(connId, (w) => { w.focusedTabId = tabId; });
}

/** Snapshot the current workspace states (test helper only). */
export function _snapshotForTests(): Map<string, WorkspaceState> {
    return get(workspaceStates);
}
