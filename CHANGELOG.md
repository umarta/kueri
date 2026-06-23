# Changelog

All notable changes to Kueri are documented here. This project adheres to
[Semantic Versioning](https://semver.org) and the spirit of
[Keep a Changelog](https://keepachangelog.com).

## [0.2.0] — 2026-06-23

The "complete the data grid" release: the grid gains every everyday operation,
the Structure tab is rebuilt TablePlus-style, and connections can now reach
production through SSL and SSH. All database differences stay behind the
`Driver` trait — the UI never branches on database type.

### Added
- **Delete rows** — multi-select (click / shift / ⌘-click), primary-key-aware `DELETE`, with a confirmation. ([#1](https://github.com/umarta/kueri/issues/1))
- **Pagination** — page large results with `LIMIT`/`OFFSET` and Prev/Next; resets on sort/filter/table change. ([#2](https://github.com/umarta/kueri/issues/2))
- **Sort by column header** — click to cycle ascending → descending → unsorted (`ORDER BY`). ([#3](https://github.com/umarta/kueri/issues/3))
- **Foreign-key navigation** — FK columns are marked; click an FK cell (↗) to open the referenced table filtered to that row. ([#4](https://github.com/umarta/kueri/issues/4))
- **Index & foreign-key management** — list/create/drop indexes and **create foreign keys** from the Structure tab (Postgres `NOT VALID` option for tables with existing data). ([#5](https://github.com/umarta/kueri/issues/5))
- **Copy rows** — multi-row selection with copy as TSV. ([#6](https://github.com/umarta/kueri/issues/6))
- **Duplicate row** (⌘D) — clone a row into a pre-filled insert form (primary keys cleared). ([#7](https://github.com/umarta/kueri/issues/7))
- **Persistent, searchable query history** — survives restarts; search and click a statement to load it into a new query tab. ([#8](https://github.com/umarta/kueri/issues/8))
- **Export** — save a result set to CSV or JSON via a native save dialog. ([#9](https://github.com/umarta/kueri/issues/9))
- **Cancel a running query** — ⌘. , a menu item, or the Cancel button shown while a query runs. ([#10](https://github.com/umarta/kueri/issues/10))
- **SSH tunnel** — optional per-connection tunnel via the system `ssh` client (key/agent auth); torn down on disconnect. ([#11](https://github.com/umarta/kueri/issues/11))
- **Find within results** — search the loaded rows with highlighting and next/previous navigation. ([#12](https://github.com/umarta/kueri/issues/12))
- **Show / hide columns** — toggle column visibility, persisted per table. ([#13](https://github.com/umarta/kueri/issues/13))
- **Read-only / safe mode** — a toolbar lock that blocks writes and DDL; defaults on for production-tagged connections. ([#14](https://github.com/umarta/kueri/issues/14))
- **Import CSV** — pick a CSV, map columns (auto-matched by name) with a preview, then batched inserts. ([#15](https://github.com/umarta/kueri/issues/15))
- **SQL formatter** — pretty-print the editor buffer with ⇧⌘F (dialect-aware). ([#16](https://github.com/umarta/kueri/issues/16))
- **SSL/TLS connection options** — SSL mode plus optional CA / client cert / client key. ([#17](https://github.com/umarta/kueri/issues/17))
- **PostgreSQL backup & restore** — `pg_dump` / `pg_restore` integration. ([#18](https://github.com/umarta/kueri/issues/18))
- **Show CREATE / DDL view** — view a table's `CREATE` statement (MySQL `SHOW CREATE`, SQLite `sqlite_master`, Postgres reconstructed). ([#19](https://github.com/umarta/kueri/issues/19))
- **TablePlus-style Structure tab** — two-pane layout: a columns grid (type / nullable / default / foreign key / comment, with primary-key chips and column search) and an indexes grid (algorithm / unique / columns / condition). Enum columns edit via a dropdown in the row editor.

### Fixed
- MySQL: tables not appearing (`information_schema` columns decoded as binary), browse/edit failing on quoted identifiers (now backticks), and `bigint unsigned` values rendering as `true`/`false`.
- Empty result sets now keep their column headers and the filter bar's column list.

## [0.1.0] — 2026-06-23

Initial public release.

### Added
- Native multi-database client for **PostgreSQL, MySQL/MariaDB and SQLite** behind a single `Driver` trait.
- Multi-connection workspaces, schema browser, and a CodeMirror SQL editor with schema-aware autocomplete (separate table and query tabs).
- Virtualized result grid with inline, primary-key-aware editing (big-integer safe) and row insertion; type-aware row-detail panel; filter bar.
- Table & column management (create / rename / drop / truncate / duplicate; add / rename / drop / alter columns) generated per dialect.
- Query log, command palette, native menu + keyboard shortcuts, Settings.
- OS keychain credential storage; signed-less release builds for macOS, Windows and Linux.

[0.2.0]: https://github.com/umarta/kueri/releases/tag/v0.2.0
[0.1.0]: https://github.com/umarta/kueri/releases/tag/v0.1.0
