import { writable } from "svelte/store";
import { api } from "../tauri";

export interface LogEntry {
  id: number;
  time: string;               // HH:MM:SS
  date: string;               // YYYY-MM-DD
  sql: string;
  ms?: number;
  rowCount?: number;          // rows returned or affected
  connectionId: string | null;
  error?: string;
}

const MAX_QUERY_LOG = 5000;

let seq = 0;

export const queryLog = writable<LogEntry[]>([]);

// ── Boot hydration ──────────────────────────────────────────────────────────
/** Call once from App.svelte onMount before startAutosave(). */
export async function initQueryLog(): Promise<void> {
  try {
    const entries = await api.loadQueryHistory();
    queryLog.set(entries);
    seq = entries.reduce((m, e) => Math.max(m, e.id), 0);
  } catch {
    // Tauri not available (browser dev) or file missing — start empty.
    queryLog.set([]);
    seq = 0;
  }
}

// ── Debounced flush ─────────────────────────────────────────────────────────
let flushTimer: ReturnType<typeof setTimeout> | null = null;

function scheduleFlush(entries: LogEntry[]) {
  if (flushTimer) clearTimeout(flushTimer);
  flushTimer = setTimeout(() => {
    api.saveQueryHistory(entries).catch(() => { /* ignore */ });
    flushTimer = null;
  }, 300);
}

function flushNow(entries: LogEntry[]) {
  if (flushTimer) { clearTimeout(flushTimer); flushTimer = null; }
  api.saveQueryHistory(entries).catch(() => { /* ignore */ });
}

// ── Timestamp ───────────────────────────────────────────────────────────────
function stamp(): { time: string; date: string } {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return {
    time: `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`,
    date: `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`,
  };
}

// ── Public API ──────────────────────────────────────────────────────────────
/** Record a SQL statement run by the user from the console. */
export function logSql(
  sql: string,
  opts: { ms?: number; rowCount?: number; connectionId?: string | null; error?: string } = {}
): void {
  let entries: LogEntry[] = [];
  queryLog.update((l) => {
    const entry: LogEntry = {
      id: ++seq,
      ...stamp(),
      sql: sql.trim(),
      ms: opts.ms,
      rowCount: opts.rowCount,
      connectionId: opts.connectionId ?? null,
      error: opts.error,
    };
    const next = [...l, entry];
    entries = next.length > MAX_QUERY_LOG ? next.slice(next.length - MAX_QUERY_LOG) : next;
    return entries;
  });
  scheduleFlush(entries);
}

export function clearLog(): void {
  queryLog.set([]);
  flushNow([]);
}

export function removeLog(id: number): void {
  let entries: LogEntry[] = [];
  queryLog.update((l) => {
    entries = l.filter((e) => e.id !== id);
    return entries;
  });
  flushNow(entries);
}

// ── Activity log (unchanged — stays in localStorage) ───────────────────────
// EVERY statement the app runs (table browses, cell edits, inserts/deletes,
// console queries…). The bottom "Query History" panel shows this; the sidebar
// "History" tab shows only console-run queries (queryLog above).
const ACT_STORAGE = "kueri.activitylog";
function loadAct(): LogEntry[] {
  try {
    const raw = localStorage.getItem(ACT_STORAGE);
    return raw ? (JSON.parse(raw) as LogEntry[]) : [];
  } catch {
    return [];
  }
}
const initialAct = loadAct();
export const activityLog = writable<LogEntry[]>(initialAct);
activityLog.subscribe((l) => {
  try {
    localStorage.setItem(ACT_STORAGE, JSON.stringify(l));
  } catch {
    /* storage unavailable / quota */
  }
});
let actSeq = initialAct.reduce((m, e) => Math.max(m, e.id), 0);

export function logActivity(sql: string, opts: { ms?: number; error?: string } = {}) {
  activityLog.update((l) => {
    const next = [
      ...l,
      { id: ++actSeq, ...stamp(), sql: sql.trim(), ms: opts.ms, connectionId: null, error: opts.error },
    ];
    return next.length > 500 ? next.slice(next.length - 500) : next;
  });
}

export function clearActivity() {
  activityLog.set([]);
}
