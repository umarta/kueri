import { writable } from "svelte/store";

export interface LogEntry {
  id: number;
  time: string; // HH:MM:SS
  date: string; // YYYY-MM-DD (for History grouping)
  sql: string;
  ms?: number;
  error?: string;
}

const STORAGE = "kueri.querylog";
const MAX = 500;

function loadPersisted(): LogEntry[] {
  try {
    const raw = localStorage.getItem(STORAGE);
    return raw ? (JSON.parse(raw) as LogEntry[]) : [];
  } catch {
    return [];
  }
}

const initial = loadPersisted();
export const queryLog = writable<LogEntry[]>(initial);

// Persist across restarts (searchable history).
queryLog.subscribe((l) => {
  try {
    localStorage.setItem(STORAGE, JSON.stringify(l));
  } catch {
    /* storage unavailable / quota */
  }
});

let seq = initial.reduce((m, e) => Math.max(m, e.id), 0);

function stamp(): { time: string; date: string } {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return {
    time: `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`,
    date: `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`,
  };
}

/** Record a SQL statement that the app executed (newest appended at the end). */
export function logSql(sql: string, opts: { ms?: number; error?: string } = {}) {
  queryLog.update((l) => {
    const next = [...l, { id: ++seq, ...stamp(), sql: sql.trim(), ms: opts.ms, error: opts.error }];
    return next.length > MAX ? next.slice(next.length - MAX) : next;
  });
}

export function clearLog() {
  queryLog.set([]);
}

export function removeLog(id: number) {
  queryLog.update((l) => l.filter((e) => e.id !== id));
}

// ── Activity log ───────────────────────────────────────────────────────────
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
    const next = [...l, { id: ++actSeq, ...stamp(), sql: sql.trim(), ms: opts.ms, error: opts.error }];
    return next.length > MAX ? next.slice(next.length - MAX) : next;
  });
}

export function clearActivity() {
  activityLog.set([]);
}
