import { writable } from "svelte/store";

export interface SavedQuery {
  id: string;
  name: string;
  sql: string;
}

const KEY = "kueri.saved";

function load(): SavedQuery[] {
  try {
    const raw = localStorage.getItem(KEY);
    return raw ? (JSON.parse(raw) as SavedQuery[]) : [];
  } catch {
    return [];
  }
}

export const savedQueries = writable<SavedQuery[]>(load());

savedQueries.subscribe((v) => {
  try {
    localStorage.setItem(KEY, JSON.stringify(v));
  } catch {
    /* storage unavailable */
  }
});

export function addSaved(name: string, sql: string) {
  savedQueries.update((l) => [{ id: crypto.randomUUID(), name: name.trim() || "Untitled", sql }, ...l]);
}

export function removeSaved(id: string) {
  savedQueries.update((l) => l.filter((q) => q.id !== id));
}
