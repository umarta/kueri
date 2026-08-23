import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";

// Mock the api before importing log.ts so the module picks up the mock.
vi.mock("../tauri", () => ({
  api: {
    loadQueryHistory: vi.fn().mockResolvedValue([]),
    saveQueryHistory: vi.fn().mockResolvedValue(undefined),
  },
}));

// Import AFTER mock is set up.
const { queryLog, logSql, removeLog, initQueryLog } = await import("./log");

describe("logSql", () => {
  beforeEach(async () => {
    await initQueryLog(); // resets store to []
  });

  it("populates rowCount and connectionId on the entry", () => {
    logSql("SELECT 1", { rowCount: 5, connectionId: "conn-abc", ms: 12 });
    const entries = get(queryLog);
    expect(entries).toHaveLength(1);
    expect(entries[0].rowCount).toBe(5);
    expect(entries[0].connectionId).toBe("conn-abc");
    expect(entries[0].ms).toBe(12);
    expect(entries[0].sql).toBe("SELECT 1");
  });

  it("trims to MAX_QUERY_LOG (5000) when over cap", async () => {
    // Pre-fill with 5000 entries using the store directly.
    const { queryLog: ql } = await import("./log");
    const fakeEntries = Array.from({ length: 5000 }, (_, i) => ({
      id: i + 1,
      time: "00:00:00",
      date: "2026-01-01",
      sql: `SELECT ${i}`,
      connectionId: null,
    }));
    ql.set(fakeEntries as any);

    logSql("SELECT overflow", { connectionId: null });
    const after = get(ql);
    expect(after).toHaveLength(5000);
    expect(after[after.length - 1].sql).toBe("SELECT overflow");
    expect(after[0].sql).toBe("SELECT 1"); // oldest dropped
  });

  it("removeLog drops the correct entry and leaves the rest", () => {
    logSql("SELECT 1", { connectionId: null });
    logSql("SELECT 2", { connectionId: null });
    logSql("SELECT 3", { connectionId: null });
    const before = get(queryLog);
    const middleId = before[1].id;
    removeLog(middleId);
    const after = get(queryLog);
    expect(after).toHaveLength(2);
    expect(after.find((e) => e.id === middleId)).toBeUndefined();
  });
});
