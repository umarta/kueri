import { api } from "../tauri";
import type { QueryResult, SafetyLevel } from "../types";
import type { ConfirmReason, RejectReason } from "./labels";

/** Thrown by runQuerySafely when the user cancels the confirmation modal. */
export class CancelledByUser extends Error {
    constructor() { super("cancelled by user"); this.name = "CancelledByUser"; }
}

export type NeedsConfirmationInfo = {
    statement: string;
    reason: ConfirmReason;
};

export type SafetyErrorInfo = {
    statement: string;
    reason: RejectReason;
    message: string;
};

/** Backend NeedsConfirmation error shape. */
type NeedsConfirmationError = {
    kind: "needs-confirmation";
    token: string;
    statement: string;
    reason: ConfirmReason;
    message: string;
};

type SafetyRejectedError = {
    kind: "safety-rejected";
    statement: string;
    reason: RejectReason;
    message: string;
};

export function isNeedsConfirmation(err: unknown): err is NeedsConfirmationError {
    return typeof err === "object" && err !== null &&
        (err as { kind?: unknown }).kind === "needs-confirmation";
}

export function isSafetyRejected(err: unknown): err is SafetyRejectedError {
    return typeof err === "object" && err !== null &&
        (err as { kind?: unknown }).kind === "safety-rejected";
}

/**
 * Wraps api.executeQuery with the Safe Mode confirmation round trip.
 * On NeedsConfirmation, awaits the caller's modal via `onNeedsConfirmation`.
 * On confirm → calls api.executeQueryConfirmed. On cancel → throws CancelledByUser.
 * On SafetyRejected → rethrows the tagged error (caller shows a toast).
 */
export async function runQuerySafely(
    connId: string,
    sql: string,
    safety: SafetyLevel,
    onNeedsConfirmation: (info: NeedsConfirmationInfo) => Promise<boolean>,
    queryId?: string,
): Promise<QueryResult> {
    const qid = queryId ?? crypto.randomUUID();
    try {
        return await api.executeQuery(connId, sql, qid, safety);
    } catch (err) {
        if (isNeedsConfirmation(err)) {
            const confirmed = await onNeedsConfirmation({
                statement: err.statement,
                reason: err.reason,
            });
            if (!confirmed) throw new CancelledByUser();
            return await api.executeQueryConfirmed(err.token, qid);
        }
        throw err;
    }
}
