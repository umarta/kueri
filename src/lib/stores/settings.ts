import { writable } from "svelte/store";

export type Theme = "auto" | "light" | "dark";

export interface Settings {
  /** Row cap applied to table browses (SELECT * ... LIMIT n). */
  rowLimit: number;
  /** Alternating row background colors in the result grid. */
  altRows: boolean;
  /** Color theme: auto follows the OS. */
  theme: Theme;
}

const KEY = "kueri.settings";
const defaults: Settings = { rowLimit: 200, altRows: true, theme: "auto" };

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
