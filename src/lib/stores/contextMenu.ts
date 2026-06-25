import { writable } from "svelte/store";

/** One entry in a right-click context menu. A separator has no label/action. */
export interface CtxItem {
  label?: string;
  action?: () => void;
  danger?: boolean;
  disabled?: boolean;
  separator?: boolean;
}

interface CtxState {
  x: number;
  y: number;
  items: CtxItem[];
}

// A single app-wide context menu, opened from anywhere via openContextMenu().
export const contextMenu = writable<CtxState | null>(null);

/** Open the context menu at the cursor with the given items. */
export function openContextMenu(e: MouseEvent, items: CtxItem[]) {
  e.preventDefault();
  e.stopPropagation();
  const visible = items.filter((i) => i.separator || i.label);
  if (visible.length === 0) return;
  contextMenu.set({ x: e.clientX, y: e.clientY, items: visible });
}

export function closeContextMenu() {
  contextMenu.set(null);
}
