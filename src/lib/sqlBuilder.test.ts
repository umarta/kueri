import { describe, it, expect } from "vitest";
import { buildRowEdits, buildUpdateSql } from "./sqlBuilder";
import type { QueryResult } from "./types";

const result: QueryResult = {
  columns: ["id", "name", "val"],
  rows: [
    [1, "Alice", "foo"],
    [2, "Bob", null],
  ],
  row_count: 2,
};

describe("buildRowEdits", () => {
  it("returns empty array for empty cellEdits", () => {
    expect(buildRowEdits({}, result)).toEqual([]);
  });

  it("converts a single cell edit to a RowEdit", () => {
    const edits = { "0:1": "Bob" };
    const [re] = buildRowEdits(edits, result);
    expect(re.rowIndex).toBe(0);
    expect(re.original).toEqual([1, "Alice", "foo"]);
    expect(re.updates).toEqual({ name: "Bob" });
  });

  it("groups multiple edits in the same row", () => {
    const edits = { "1:1": "Charlie", "1:2": null };
    const [re] = buildRowEdits(edits, result);
    expect(re.rowIndex).toBe(1);
    expect(re.updates).toEqual({ name: "Charlie", val: null });
  });

  it("produces one RowEdit per modified row", () => {
    const edits = { "0:0": "10", "1:0": "20" };
    const res = buildRowEdits(edits, result);
    expect(res).toHaveLength(2);
    expect(res.map((r) => r.rowIndex).sort()).toEqual([0, 1]);
  });
});

describe("buildUpdateSql", () => {
  const tbl = { schema: "public", table: "users" };

  it("generates UPDATE with pk WHERE clause (postgres quoting)", () => {
    const rowEdits = buildRowEdits({ "0:1": "Bob" }, result);
    const sql = buildUpdateSql(rowEdits, result.columns, ["id"], tbl, "postgres");
    expect(sql).toBe(
      'UPDATE "public"."users" SET "name" = \'Bob\' WHERE "id" = 1;'
    );
  });

  it("uses backtick quoting for mysql", () => {
    const rowEdits = buildRowEdits({ "0:1": "Bob" }, result);
    const sql = buildUpdateSql(rowEdits, result.columns, ["id"], tbl, "mysql");
    expect(sql).toContain("`public`.`users`");
    expect(sql).toContain("`name` = 'Bob'");
    expect(sql).toContain("`id` = 1");
  });

  it("emits NULL literal for null value", () => {
    const rowEdits = buildRowEdits({ "0:2": null }, result);
    const sql = buildUpdateSql(rowEdits, result.columns, ["id"], tbl, "postgres");
    expect(sql).toContain('"val" = NULL');
  });

  it("uses IS NULL in WHERE for null pk value", () => {
    const rowEdits = buildRowEdits({ "1:1": "X" }, result);
    const sql = buildUpdateSql(rowEdits, result.columns, ["val"], tbl, "postgres");
    expect(sql).toContain('"val" IS NULL');
  });

  it("falls back to all columns in WHERE when pkColumns is empty", () => {
    const rowEdits = buildRowEdits({ "0:1": "Bob" }, result);
    const sql = buildUpdateSql(rowEdits, result.columns, [], tbl, "postgres");
    expect(sql).toContain('"id" =');
    expect(sql).toContain('"name" =');
    expect(sql).toContain('"val" =');
  });

  it("joins multiple row edits with newlines", () => {
    const edits = { "0:1": "Alice2", "1:1": "Bob2" };
    const rowEdits = buildRowEdits(edits, result);
    const sql = buildUpdateSql(rowEdits, result.columns, ["id"], tbl, "postgres");
    expect(sql.split("\n")).toHaveLength(2);
  });
});
