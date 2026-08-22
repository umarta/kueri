import type { SafetyLevel } from "../types";

export type ConfirmReason = "destructive-no-where" | "write" | "ddl";
export type RejectReason = "read-only-mode";

export function humanLabel(level: SafetyLevel): string {
    return {
        "off": "Off",
        "warn": "Warn",
        "confirm-destructive": "Confirm destructive",
        "confirm-writes": "Confirm writes",
        "confirm-ddl": "Confirm DDL",
        "read-only": "Read-only",
    }[level];
}

export function railColor(level: SafetyLevel): string {
    return {
        "off": "transparent",
        "warn": "var(--color-gray-500, #6b7280)",
        "confirm-destructive": "var(--color-yellow-500, #eab308)",
        "confirm-writes": "var(--color-orange-500, #f97316)",
        "confirm-ddl": "var(--color-orange-500, #f97316)",
        "read-only": "var(--color-red-500, #ef4444)",
    }[level];
}

export function bannerText(level: SafetyLevel): string {
    return {
        "off": "",
        "warn": "Safety warnings only — no confirmations required.",
        "confirm-destructive": "Destructive statements (DELETE / UPDATE without WHERE) will prompt to confirm.",
        "confirm-writes": "All write statements will prompt to confirm.",
        "confirm-ddl": "Write and DDL statements will prompt to confirm.",
        "read-only": "Read-only mode active. Only SELECT statements are allowed.",
    }[level];
}

export function reasonText(reason: ConfirmReason | RejectReason): string {
    return {
        "destructive-no-where": "This statement modifies data without a WHERE clause.",
        "write": "This statement modifies data.",
        "ddl": "This statement changes the database schema.",
        "read-only-mode": "Read-only mode is active. Only SELECT statements are allowed.",
    }[reason];
}
