# Kueri — Feature Backlog

Where Kueri stands today and what's needed to make it a confident daily-driver,
benchmarked against TablePlus / DataGrip / Postico.

**Already shipped:** multi-DB (Postgres/MySQL/SQLite), multi-connection workspaces,
schema browser, SQL editor with **schema-aware autocomplete**, table vs query tabs,
virtualized grid with inline cell editing (PK-aware, big-int safe), editable
single-table query results, **insert row**, type-aware row-detail panel, filter bar,
table & column management (create/rename/drop/truncate/duplicate + add/rename/drop/alter
columns + defaults), query log, command palette, native menu + shortcuts, settings,
keychain credential storage, and CI/CD release builds for macOS/Windows/Linux.

Priority: **P0** = blocks daily use · **P1** = important · **P2** = nice to have ·
**Future** = larger / external. Scope: **S** ≤1 day · **M** a few days · **L** 1–2 weeks.

---

## P0 — Essential (daily-driver blockers)

- [ ] **Delete row(s)** · S · frontend+reuse — You can insert and edit but not delete; core CRUD is incomplete. Select row(s) → `DELETE` with a PK-aware `WHERE` + confirm.
- [ ] **Pagination / load more** · M · frontend+backend — Results are capped at the row limit with no way past it, so large tables are unusable. Offset paging or infinite scroll, with the live row count.
- [ ] **Sort by clicking a column header** · S · frontend — Toggle asc/desc → `ORDER BY`; a basic table expectation that's currently missing.
- [ ] **Export results & tables** · M · frontend(+backend for streaming) — CSV / JSON (and SQL `INSERT`) for a result set or a whole table. One of the most common day-to-day needs.
- [ ] **Cancel a running query** (`⌘.`) · M · backend — Long queries freeze the workflow with no escape. Needs a per-connection cancel handle on the Rust side.

## P1 — Important

- [ ] **SSH tunnel** · L · backend — Most production databases are only reachable through a bastion; without this Kueri can't connect to them at all.
- [ ] **Foreign-key navigation** · M · backend+frontend — Click a FK cell → jump to the referenced row. A signature productivity feature for relational work.
- [ ] **Indexes / foreign keys / constraints management** · M — View and create/drop indexes, FKs, unique/check constraints in the Structure tab (today it only handles columns).
- [ ] **Copy / paste rows & cells** · S — Copy as TSV/CSV/JSON/`INSERT`; paste to insert/update. Pairs with delete/duplicate.
- [ ] **Duplicate row** (`⌘D`) · S — Clone a row into the insert form.
- [ ] **Query history** · S — Persisted, searchable history of executed statements with one-click re-run (the log panel is in-memory only).
- [ ] **Find within results** (`⌘F` in grid) · S — Highlight/jump to matching cells in the loaded result.
- [ ] **Column resize / reorder / show-hide** · M — Per-table column layout, persisted.
- [ ] **Read-only / safe mode** · S — Per-connection guard that blocks writes (and/or confirms) on production, keyed to the environment tag.
- [ ] **Import CSV** · M — Map columns → insert into a table.
- [ ] **SQL formatter** · S — Prettify the editor buffer (dialect-aware).
- [ ] **SSL/TLS connection options** · S · backend — Cert/key/CA file paths + verify mode for cloud databases that require TLS.
- [ ] **Confirm before delete; surface affected-row counts** · S — Safety for destructive grid actions.

## P2 — Nice to have

- [ ] **Browse views, functions, triggers, sequences, enums** · M — Beyond tables; with a "Show CREATE / DDL" view per object.
- [ ] **`EXPLAIN` / query plan viewer** · M — Run and visualize plans.
- [ ] **Saved / named queries (snippets)** · S — Reusable queries per connection.
- [ ] **Light theme + theme toggle** · M — Currently dark-only; add a light token set and an Auto/Light/Dark setting.
- [ ] **Window state persistence** · S — Remember size/position and last open workspace/tabs.
- [ ] **In-app auto-update** · M · backend — `tauri-plugin-updater` against the GitHub releases (currently manual download).
- [ ] **Connection groups / folders + reorder** · S — Organize many saved connections.
- [ ] **Multiple result sets per query** · M — Show each statement's result when running a batch.
- [ ] **Generate SQL from a table** · S — `SELECT` / `INSERT` / `CREATE` templates from the schema.
- [ ] **Per-cell value viewer** · S — Expand long text/JSON/blob in a modal (the row-detail JSON editor is a start).

## Future / larger

- [ ] **SQL Server driver** (`tiberius`) · L — promote the stub to a real driver.
- [ ] **NoSQL mode** · L — Redis key browser and MongoDB document view (separate, non-tabular UI as documented in `db/nosql.rs`).
- [ ] **More engines** — ClickHouse, CockroachDB, DuckDB, Oracle.
- [ ] **ER diagram** · L — Visual schema with relationships.
- [ ] **Backup / restore** · L — `pg_dump`/`mysqldump`-style export & import.
- [ ] **Code signing & notarization** · M — Signed macOS (notarized) + Windows builds to remove the unsigned-app warnings.
- [ ] **Sentry/telemetry (opt-in)** · S — Crash + error reporting for the desktop app.

---

### Suggested next milestone (v0.2 — "complete the data grid")

`Delete row` + `Pagination` + `Sort by column` + `Export CSV/JSON` + `Copy rows`.
Together these close the most glaring everyday gaps and make the grid feel finished.
