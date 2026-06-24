# Changelog

All notable changes to Kueri are documented here. This project adheres to
[Semantic Versioning](https://semver.org) and the spirit of
[Keep a Changelog](https://keepachangelog.com).

## [0.4.0] — 2026-06-25

Serious-client features and a much richer editing experience.

### Added
- **Manual transactions** — Begin / Commit / Rollback from the toolbar, on a pinned connection; a TXN badge shows the open state. (#56)
- **Native SQL export** — Plain-SQL dumps are generated in-app over the connection, so they never hit a `pg_dump` client/server version mismatch. (#66)
- **Matching client tools, automatically** — for the binary Custom (`.dump`) format, Kueri detects every installed PostgreSQL client version, auto-selects the one matching the server, and offers to install it (Homebrew) or open the download page if none matches. A manual *Settings → Client tools folder* still overrides. (#67)
- **Type-aware temporal editors** — `date` (date-only), `datetime`/`timestamp`, and `timestamp with time zone` columns get a calendar **date picker** in both the grid and the row-detail panel; tz columns also get a **human-labelled timezone selector** (e.g. “Jakarta (WIB) · Bangkok (UTC+07:00)”). (#100, #105, #106, #107)
- **Synced editing** — staged edits are shared between the grid and the row-detail panel, so a change in either surface shows in both and ⌘S commits all of them. (#107)
- **Activity log** — the bottom “Query History” panel now records everything the app runs (browses, edits, inserts/deletes, console), kept separate from the sidebar **History** of console-only runs. (#107)
- **Global close button** — every modal has a consistent close (✕). (#104)
- **Row-detail toggle** — a setting controls whether selecting a row auto-opens the detail panel. (#103)

### Fixed
- **Linux blank-window / WebKitGTK crash** — disable the DMABUF renderer at startup; README clarifies runtime vs build dependencies (prefer `.deb`/`.rpm`). (#98)
- **Export dialog** — `.field` CSS collision squished the layout; date-picker inputs now match the app theme (dark/light) instead of a white box. (#64)
- README rewritten with screenshots + a repeatable Playwright capture tool.

## [0.3.0] — 2026-06-24

A big step toward TablePlus/Navicat parity.

### Added
- **Keyboard grid navigation & editing** — arrows / Tab / Enter / type-to-edit, focus ring, ⌘C. (#28)
- **Run multiple statements** in one script, with a result-set switcher. (#29)
- **Cancel a running query** (⌘.). (#10)
- **Export the whole table** + **SQL `INSERT`** format (CSV / JSON / SQL). (#30)
- **Import CSV** into a table (column mapping + preview). (#15)
- **Cell value inspector** + **aggregate stats** (Σ/avg/min/max on a selection). (#35, #37)
- **Saved queries / snippets** and **persistent, date-grouped query History** with a right-click menu (Run / Copy / Insert / Add to Queries / Delete). (#33, #8)
- **Generate SQL** from a table (SELECT/INSERT/UPDATE/CREATE) and **EXPLAIN**. (#32, #34)
- **3-tab sidebar** — Items (tables/views, expandable to columns), Queries, History.
- **Foreign-key navigation**, **index management + FK creation** (Postgres `NOT VALID`), **read-only mode**. (#4, #5, #14)
- **Find & Replace** in the SQL editor. (#61)
- **Server Monitor** — running sessions + Kill, and a **user/role list**. (#57, #58)
- **Create / drop database & schema**. (#59)
- **Column comments** shown in the Structure tab. (#60)
- **Light theme** + Auto/Light/Dark. (#38)
- **MySQL & SQLite backup/restore** (parity with Postgres). (#39)
- **SSH tunnel** and **SSL/TLS** connection options; **connection groups**. (#11, #17, #36)
- **Preview tabs** (single-click reuses, double-click pins) and **session restore** (reopen connections on launch). (#62)
- TablePlus-style **two-pane Structure tab**, **enum dropdowns**, **show CREATE / DDL**, **duplicate row**, **sort / paginate / find / show-hide columns / copy / delete**.

### Fixed
- MySQL: tables not listing, backtick quoting for browse/edit, `bigint unsigned` shown as `true/false`.
- Empty result sets keep their column headers + filter columns.
- History records only console-run statements (not the grid's own SQL); clicking a History/Saved item loads into the active editor instead of spawning tabs.
- Release pipeline no longer drops platform assets (create → build → publish).

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

[0.4.0]: https://github.com/umarta/kueri/releases/tag/v0.4.0
[0.3.0]: https://github.com/umarta/kueri/releases/tag/v0.3.0
[0.2.0]: https://github.com/umarta/kueri/releases/tag/v0.2.0
[0.1.0]: https://github.com/umarta/kueri/releases/tag/v0.1.0
