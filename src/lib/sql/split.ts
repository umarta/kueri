export interface Statement {
  text: string;
  from: number;
  to: number;
}

export function splitStatements(sql: string): Statement[] {
  const stmts: Statement[] = [];
  let start = 0;
  let i = 0;

  while (i < sql.length) {
    const ch = sql[i];
    if (ch === "'") {
      i++;
      while (i < sql.length) {
        if (sql[i] === "'" && sql[i + 1] === "'") { i += 2; continue; }
        if (sql[i] === "'") { i++; break; }
        i++;
      }
    } else if (ch === '"') {
      i++;
      while (i < sql.length && sql[i] !== '"') i++;
      if (i < sql.length) i++;
    } else if (ch === '-' && sql[i + 1] === '-') {
      while (i < sql.length && sql[i] !== '\n') i++;
    } else if (ch === '/' && sql[i + 1] === '*') {
      i += 2;
      while (i < sql.length && !(sql[i] === '*' && sql[i + 1] === '/')) i++;
      if (i < sql.length) i += 2;
    } else if (ch === ';') {
      const segEnd = i + 1;
      const text = sql.slice(start, segEnd).trim();
      if (text && text !== ';') stmts.push({ text, from: start, to: segEnd });
      i++;
      // Skip whitespace to find the start of the next statement
      while (i < sql.length && /\s/.test(sql[i])) i++;
      start = i;
    } else {
      i++;
    }
  }

  const tail = sql.slice(start).trim();
  if (tail) stmts.push({ text: tail, from: start, to: sql.length });
  return stmts;
}
