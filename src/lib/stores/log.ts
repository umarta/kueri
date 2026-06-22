import { writable } from "svelte/store";

export interface LogEntry {
  id: number;
  time: string; // HH:MM:SS
  sql: string;
  ms?: number;
  error?: string;
}

export const queryLog = writable<LogEntry[]>([]);

let seq = 0;
const MAX = 500;

function clock(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

/** Record a SQL statement that the app executed (newest appended at the end). */
export function logSql(sql: string, opts: { ms?: number; error?: string } = {}) {
  queryLog.update((l) => {
    const next = [...l, { id: ++seq, time: clock(), sql: sql.trim(), ms: opts.ms, error: opts.error }];
    return next.length > MAX ? next.slice(next.length - MAX) : next;
  });
}

export function clearLog() {
  queryLog.set([]);
}
