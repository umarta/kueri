import { describe, it, expect } from "vitest";
import { splitStatements } from "./split";

describe("splitStatements", () => {
  it("splits two statements", () => {
    const stmts = splitStatements("SELECT 1; SELECT 2;");
    expect(stmts).toHaveLength(2);
    expect(stmts[0].text).toBe("SELECT 1;");
    expect(stmts[1].text).toBe("SELECT 2;");
  });

  it("does not split on semicolon inside single-quoted string", () => {
    const stmts = splitStatements("SELECT 'a;b' FROM t;");
    expect(stmts).toHaveLength(1);
    expect(stmts[0].text).toBe("SELECT 'a;b' FROM t;");
  });

  it("does not split on semicolon inside block comment", () => {
    const stmts = splitStatements("SELECT /* a;b */ 1;");
    expect(stmts).toHaveLength(1);
  });

  it("drops trailing empty segment after last semicolon", () => {
    const stmts = splitStatements("SELECT 1;   ");
    expect(stmts).toHaveLength(1);
  });

  it("handles a trailing statement without semicolon", () => {
    const stmts = splitStatements("SELECT 1; SELECT 2");
    expect(stmts).toHaveLength(2);
    expect(stmts[1].text).toBe("SELECT 2");
  });

  it("returns empty array for empty string", () => {
    expect(splitStatements("")).toHaveLength(0);
  });

  it("returns empty array for whitespace only", () => {
    expect(splitStatements("   \n   ")).toHaveLength(0);
  });

  it("reports correct from/to offsets", () => {
    const stmts = splitStatements("SELECT 1; SELECT 2;");
    expect(stmts[0].from).toBe(0);
    expect(stmts[0].to).toBe(9);
    expect(stmts[1].from).toBe(10);
    expect(stmts[1].to).toBe(19);
  });

  it("finds statement containing a cursor position", () => {
    const stmts = splitStatements("SELECT 1; SELECT 2;");
    const cursor = 12; // inside "SELECT 2"
    const hit = stmts.find((s) => s.from <= cursor && cursor <= s.to);
    expect(hit?.text).toBe("SELECT 2;");
  });
});
