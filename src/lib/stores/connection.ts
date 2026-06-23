import { writable, get } from "svelte/store";
import type { ConnectionConfig } from "../types";
import { api } from "../tauri";

export const activeConnectionId = writable<string | null>(null);

/** The config behind the active connection — drives toolbar identity (name, env, db). */
export const activeConnection = writable<ConnectionConfig | null>(null);

/** An open connection ("workspace"). The backend keeps each alive in its pool;
 *  the left rail switches which one is active. */
export interface Workspace {
  id: string;
  config: ConnectionConfig;
}
export const workspaces = writable<Workspace[]>([]);

/** The schema currently selected in the sidebar — used to resolve unqualified
 *  table names in query-tab SQL (e.g. `SELECT * FROM orders`). */
export const activeSchema = writable<string>("");

/** Read-only / safe mode for the active connection — blocks writes & DDL.
 *  Defaults on for production-tagged connections. */
export const readOnly = writable<boolean>(false);

/** A statement is "read-only" if it can't modify data or schema. */
export function isReadStatement(sql: string): boolean {
  const s = sql.replace(/\/\*[\s\S]*?\*\//g, " ").replace(/--[^\n]*/g, " ").trim().toLowerCase();
  return /^(select|with|show|explain|describe|desc|pragma|table|values)\b/.test(s);
}

/** Whether a connection should start in read-only mode (production safety). */
export function shouldStartReadOnly(color?: string, tag?: string): boolean {
  return color === "prod" || /prod/i.test(tag ?? "");
}

/**
 * Schema catalog for editor autocomplete: table name → known column names.
 * Populated cheaply as the user browses; reset on disconnect.
 */
export const schemaCatalog = writable<Record<string, string[]>>({});

export function catalogTables(tables: string[]) {
  schemaCatalog.update((cat) => {
    const next = { ...cat };
    for (const t of tables) if (!next[t]) next[t] = [];
    return next;
  });
}

export function catalogColumns(table: string, columns: string[]) {
  if (!table || columns.length === 0) return;
  schemaCatalog.update((cat) => ({ ...cat, [table]: columns }));
}

// ── Saved connections ─────────────────────────────────────────────────────────
//
// Connections (without passwords) persist to a JSON file in the app config dir
// via Tauri commands; passwords live in the OS keychain keyed by connection id.
// Outside Tauri (browser dev), it falls back to localStorage and in-memory
// passwords so the UI still works.

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const LS_KEY = "kueri.connections";

export const savedConnections = writable<ConnectionConfig[]>([]);

// Initial load (async — the store fills in once it resolves).
(async () => {
  try {
    if (isTauri) {
      savedConnections.set(await api.loadConnections());
    } else {
      const raw = localStorage.getItem(LS_KEY);
      if (raw) savedConnections.set(JSON.parse(raw));
    }
  } catch {
    /* no saved connections / storage unavailable */
  }
})();

function persist(list: ConnectionConfig[]) {
  // Never write passwords to disk — those go to the keychain.
  const safe = list.map(({ password: _pw, ...rest }) => rest);
  if (isTauri) {
    api.saveConnections(safe).catch(() => {});
  } else {
    try {
      localStorage.setItem(LS_KEY, JSON.stringify(safe));
    } catch {
      /* ignore */
    }
  }
}

/** Insert or update a connection, persist it, and stash its password in the keychain. */
export async function upsertConnection(conn: ConnectionConfig) {
  savedConnections.update((list) => {
    const i = list.findIndex((c) => c.id === conn.id);
    if (i === -1) return [...list, conn];
    const next = list.slice();
    next[i] = conn;
    return next;
  });
  persist(get(savedConnections));
  if (isTauri && conn.password) {
    try {
      await api.secretSet(conn.id, conn.password);
    } catch {
      /* keychain unavailable — password stays in-memory for the session */
    }
  }
}

export async function removeConnection(id: string) {
  savedConnections.update((list) => list.filter((c) => c.id !== id));
  persist(get(savedConnections));
  if (isTauri) {
    try {
      await api.secretDelete(id);
    } catch {
      /* ignore */
    }
  }
}

/** Resolve a connection's password: in-memory if present, else from the keychain. */
export async function resolvePassword(conn: ConnectionConfig): Promise<string> {
  if (conn.password) return conn.password;
  if (isTauri) {
    try {
      return (await api.secretGet(conn.id)) ?? "";
    } catch {
      return "";
    }
  }
  return "";
}
