import { writable } from "svelte/store";
import type { ConfirmReason } from "./labels";

export type SafetyPrompt = {
    statement: string;
    reason: ConfirmReason;
    resolve: (ok: boolean) => void;
};

export const safetyPrompt = writable<SafetyPrompt | null>(null);

/** Show the modal and await the user's decision. Any caller can invoke this. */
export function showSafetyModal(info: { statement: string; reason: ConfirmReason }): Promise<boolean> {
    return new Promise((resolve) => {
        safetyPrompt.set({ statement: info.statement, reason: info.reason, resolve });
    });
}
