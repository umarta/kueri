import { describe, it, expect } from "vitest";
import { parseParams, substituteParams } from "./params";

describe("parseParams", () => {
  it("finds a single param", () => {
    const refs = parseParams("SELECT :id");
    expect(refs).toEqual([{ name: "id", from: 7, to: 10 }]);
  });

  it("skips params inside single-quoted strings", () => {
    const refs = parseParams("SELECT ':name' FROM t");
    expect(refs).toHaveLength(0);
  });

  it("skips params inside double-quoted identifiers", () => {
    const refs = parseParams('SELECT ":col" FROM t');
    expect(refs).toHaveLength(0);
  });

  it("skips params inside line comments", () => {
    const refs = parseParams("SELECT 1 -- :skip\nFROM t");
    expect(refs).toHaveLength(0);
  });

  it("skips params inside block comments", () => {
    const refs = parseParams("SELECT /* :skip */ 1");
    expect(refs).toHaveLength(0);
  });

  it("finds repeated param names as separate refs", () => {
    const refs = parseParams("SELECT :x, :x");
    expect(refs).toHaveLength(2);
    expect(refs[0].name).toBe("x");
    expect(refs[1].name).toBe("x");
  });

  it("finds multiple distinct params", () => {
    const refs = parseParams("SELECT :a, :b, :c");
    expect(refs.map((r) => r.name)).toEqual(["a", "b", "c"]);
  });

  it("ignores bare colons (e.g. cast syntax)", () => {
    const refs = parseParams("SELECT 1::int");
    expect(refs).toHaveLength(0);
  });
});

describe("substituteParams", () => {
  it("replaces :name with $1 for postgres", () => {
    const { sql, ordered } = substituteParams("SELECT :id", "postgres", []);
    expect(sql).toBe("SELECT $1");
    expect(ordered).toEqual(["id"]);
  });

  it("reuses $N for repeated postgres params", () => {
    const { sql, ordered } = substituteParams("SELECT :x, :x", "postgres", []);
    expect(sql).toBe("SELECT $1, $1");
    expect(ordered).toEqual(["x"]);
  });

  it("replaces :name with ? for mysql", () => {
    const { sql, ordered } = substituteParams("SELECT :id", "mysql", []);
    expect(sql).toBe("SELECT ?");
    expect(ordered).toEqual(["id"]);
  });

  it("repeats ? for repeated mysql params", () => {
    const { sql, ordered } = substituteParams("SELECT :x, :x", "mysql", []);
    expect(sql).toBe("SELECT ?, ?");
    expect(ordered).toEqual(["x", "x"]);
  });

  it("returns original sql when no params", () => {
    const { sql, ordered } = substituteParams("SELECT 1", "postgres", []);
    expect(sql).toBe("SELECT 1");
    expect(ordered).toHaveLength(0);
  });
});
