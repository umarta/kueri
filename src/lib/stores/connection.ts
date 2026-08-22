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

/** A statement is "read-only" if it can't modify data or schema. */
export function isReadStatement(sql: string): boolean {
  const s = sql.replace(/\/\*[\s\S]*?\*\//g, " ").replace(/--[^\n]*/g, " ").trim().toLowerCase();
  return /^(select|with|show|explain|describe|desc|pragma|table|values)\b/.test(s);
}

/** Whether a connection should start in read-only mode (production safety). */
export function shouldStartReadOnly(color?: string, tag?: string): boolean {
  return color === "prod" || /prod/i.test(tag ?? "");
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
  // password is now PasswordSource (a tagged enum), never plaintext — safe to serialize.
  if (isTauri) {
    api.saveConnections(list).catch(() => {});
  } else {
    try {
      localStorage.setItem(LS_KEY, JSON.stringify(list));
    } catch {
      /* ignore */
    }
  }
}

/** Insert or update a connection, persist it, and stash its password in the keychain.
 *  Pass `plaintext` (from a form input) to write the password to the keychain.
 *  The `conn.password` field should already be `{ kind: "keychain" }` when a
 *  plaintext string is provided.
 */
export async function upsertConnection(conn: ConnectionConfig, plaintext?: string) {
  savedConnections.update((list) => {
    const i = list.findIndex((c) => c.id === conn.id);
    if (i === -1) return [...list, conn];
    const next = list.slice();
    next[i] = conn;
    return next;
  });
  persist(get(savedConnections));
  if (isTauri && plaintext) {
    try {
      await api.secretSet(conn.id, plaintext);
    } catch {
      /* keychain unavailable */
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

/** Resolve a connection's password from the keychain. */
export async function resolvePassword(conn: ConnectionConfig): Promise<string> {
  if (isTauri) {
    try {
      return (await api.secretGet(conn.id)) ?? "";
    } catch {
      return "";
    }
  }
  return "";
}
