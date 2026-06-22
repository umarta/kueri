import { writable } from "svelte/store";

export interface Settings {
  /** Row cap applied to table browses (SELECT * ... LIMIT n). */
  rowLimit: number;
  /** Alternating row background colors in the result grid. */
  altRows: boolean;
}

const KEY = "kueri.settings";
const defaults: Settings = { rowLimit: 200, altRows: true };

function load(): Settings {
  try {
    return { ...defaults, ...JSON.parse(localStorage.getItem(KEY) || "{}") };
  } catch {
    return { ...defaults };
  }
}

export const settings = writable<Settings>(load());

settings.subscribe((v) => {
  try {
    localStorage.setItem(KEY, JSON.stringify(v));
  } catch {
    /* storage unavailable */
  }
});
