export interface ParamRef {
  name: string;
  from: number;
  to: number;
}

export function parseParams(sql: string): ParamRef[] {
  const refs: ParamRef[] = [];
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
    } else if (ch === ':') {
      // Skip postgres cast (::type)
      if (sql[i + 1] === ':') { i += 2; continue; }
      const start = i;
      i++;
      let name = '';
      while (i < sql.length && /[A-Za-z0-9_]/.test(sql[i])) {
        name += sql[i++];
      }
      if (name && /^[A-Za-z_]/.test(name)) {
        refs.push({ name, from: start, to: start + 1 + name.length });
      }
    } else {
      i++;
    }
  }
  return refs;
}

export function substituteParams(
  sql: string,
  dialect: "postgres" | "mysql" | "sqlite",
  _order: string[]
): { sql: string; ordered: string[] } {
  const refs = parseParams(sql);
  if (refs.length === 0) return { sql, ordered: [] };

  const ordered: string[] = [];
  const nameToIdx = new Map<string, number>();
  let result = '';
  let last = 0;

  for (const ref of refs) {
    result += sql.slice(last, ref.from);
    if (dialect === 'postgres') {
      if (!nameToIdx.has(ref.name)) {
        nameToIdx.set(ref.name, ordered.length);
        ordered.push(ref.name);
      }
      result += `$${nameToIdx.get(ref.name)! + 1}`;
    } else {
      ordered.push(ref.name);
      result += '?';
    }
    last = ref.to;
  }
  result += sql.slice(last);
  return { sql: result, ordered };
}
