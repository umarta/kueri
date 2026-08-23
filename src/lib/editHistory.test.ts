import { describe, it, expect } from "vitest";
import { pushEdit, applyUndo } from "./editHistory";

describe("pushEdit", () => {
  it("records undefined prev for a key not yet in edits", () => {
    const history = pushEdit([], "0:1", {});
    expect(history).toEqual([{ key: "0:1", prev: undefined }]);
  });

  it("records previous value for an existing key", () => {
    const history = pushEdit([], "0:1", { "0:1": "old" });
    expect(history).toEqual([{ key: "0:1", prev: "old" }]);
  });

  it("records null prev when existing value is null", () => {
    const history = pushEdit([], "0:1", { "0:1": null });
    expect(history).toEqual([{ key: "0:1", prev: null }]);
  });

  it("does not mutate the original history array", () => {
    const original: import("./editHistory").EditAction[] = [];
    pushEdit(original, "0:0", {});
    expect(original).toHaveLength(0);
  });
});

describe("applyUndo", () => {
  it("is a no-op on empty history", () => {
    const edits = { "0:0": "x" };
    const result = applyUndo([], edits);
    expect(result.history).toEqual([]);
    expect(result.edits).toEqual({ "0:0": "x" });
  });

  it("restores a previous string value", () => {
    const history = [{ key: "0:0", prev: "old" as string | null | undefined }];
    const edits = { "0:0": "new" };
    const result = applyUndo(history, edits);
    expect(result.history).toEqual([]);
    expect(result.edits).toEqual({ "0:0": "old" });
  });

  it("restores null for a cell that was null before", () => {
    const history = [{ key: "0:0", prev: null as string | null | undefined }];
    const edits = { "0:0": "new" };
    const result = applyUndo(history, edits);
    expect(result.edits["0:0"]).toBeNull();
  });

  it("removes key when prev was undefined (cell was clean)", () => {
    const history = [{ key: "0:0", prev: undefined as string | null | undefined }];
    const edits = { "0:0": "new" };
    const result = applyUndo(history, edits);
    expect("0:0" in result.edits).toBe(false);
  });

  it("does not mutate original history or edits", () => {
    const history = [{ key: "0:0", prev: "old" as string | null | undefined }];
    const edits = { "0:0": "new" };
    applyUndo(history, edits);
    expect(history).toHaveLength(1);
    expect(edits["0:0"]).toBe("new");
  });
});
