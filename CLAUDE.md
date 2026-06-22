# CLAUDE.md — Kueri

Guidance for working on this repo with Claude Code. Read this first.

## What this is
Kueri is a lightweight, native multi-database GUI client. Goal: TablePlus-grade
*simplicity and speed*, open source. The complexity of supporting many databases
is hidden behind one abstraction so the UI stays simple.

Hard rule: **keep the UI dumb about which database it's talking to.** All
backend differences live behind the `Driver` trait. Components and Tauri
commands never branch on database type.

## Stack
- Backend: Rust + Tauri v2 (`src-tauri/`)
- DB layer: `sqlx` 0.8 (postgres, mysql, sqlite) behind a `Driver` trait
- Frontend: Svelte 4 + TypeScript + Vite (`src/`)

## Architecture (data flow)
```
Svelte component → api.* (src/lib/tauri.ts) → invoke()
   → Tauri command (src-tauri/src/commands.rs)
   → db::open(cfg) picks a driver by DbKind
   → Box<dyn Driver> (postgres / mysql / sqlite / ...)
   → returns the SAME shape (SchemaInfo / TableInfo / QueryResult)
```
The `Driver` trait (`src-tauri/src/db/driver.rs`) is the seam. Live connections
are stored as `Arc<dyn Driver>` in `AppState` (`db/pool.rs`), keyed by id.

## Database support status
| DB | Status | Backend |
|----|--------|---------|
| PostgreSQL | ✅ implemented | sqlx |
| MySQL / MariaDB | ✅ implemented | sqlx |
| SQLite | ✅ implemented | sqlx |
| SQL Server | ⛔ stub | needs `tiberius` (sqlx dropped MSSQL) — see db/sqlserver.rs |
| Redis | ⛔ stub | needs `redis` + separate non-tabular UI — see db/nosql.rs |
| MongoDB | ⛔ stub | needs `mongodb` + separate document UI — see db/nosql.rs |

NoSQL note: Redis/Mongo do NOT fit the schema→table→SQL model. Do not force
them into the grid; they need their own UI mode. The stub explains the design.

## Key files
- `src-tauri/src/db/driver.rs`   — the `Driver` trait + shared result types
- `src-tauri/src/db/mod.rs`      — `DbKind` enum + `open()` factory
- `src-tauri/src/db/postgres.rs` — reference driver implementation
- `src-tauri/src/db/{mysql,sqlite}.rs` — sqlx drivers
- `src-tauri/src/db/{sqlserver,nosql}.rs` — stubs with integration plans
- `src-tauri/src/db/connect.rs`  — ConnectionConfig + per-kind URL builders
- `src-tauri/src/db/pool.rs`     — Arc<dyn Driver> registry (AppState)
- `src-tauri/src/commands.rs`    — Tauri commands (DB-agnostic)
- `src/lib/tauri.ts`             — typed command wrappers; ADD NEW COMMANDS HERE
- `src/components/*.svelte`      — UI (also DB-agnostic)

## Adding a new relational driver (the pattern)
1. Create `src-tauri/src/db/<name>.rs` with a struct holding the pool/client.
2. `impl Driver for YourDriver` (async_trait): list_schemas, list_tables,
   list_columns, run_query, close.
3. Add a `DbKind` variant and wire it in `db::mod.rs::open()`.
4. Add the variant to `DbKind` in `src/lib/types.ts` and to the picker list in
   `ConnectionForm.svelte`.
No UI or command changes needed beyond the picker.

## Conventions
- Rust: `cargo fmt`; errors via `AppError`/`AppResult`. Propagate with `?`.
- TS: strict; all backend calls go through `api` in `src/lib/tauri.ts`.
- Keyboard-first UX is a feature (⌘↵ runs SQL). Wire shortcuts early.

## Run
```bash
npm install
npm run tauri dev    # first build is slow (Tauri + sqlx with 3 DB drivers)
```

## Roadmap (MVP order — do top-down)
1. [done] Driver abstraction + Postgres/MySQL/SQLite
2. [ ] Persist saved connections (@tauri-apps/plugin-store)
3. [ ] Store passwords in OS keychain, not plaintext
4. [ ] CodeMirror 6 editor + schema-aware autocomplete (dialect per DbKind)
5. [ ] Virtualized grid (@tanstack/svelte-virtual) for large results
6. [ ] Inline cell edit → stage diff → commit (the "TablePlus feel")
7. [ ] SQL Server driver (tiberius) — see db/sqlserver.rs
8. [ ] SSH tunnel support
9. [ ] Multiple query tabs (⌘T)
10. [ ] NoSQL mode (Redis key browser, Mongo document view) — separate UI

## Known gaps / TODO markers
- Per-driver `decode()` covers common types; exotic types fall back to text/null.
- No connection persistence yet — connections reset on restart.
- Grid is a plain HTML table — virtualize before large datasets.
- SQL Server + NoSQL are stubs that return an explicit "not implemented" error.
