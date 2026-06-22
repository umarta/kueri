# Kueri

**A lightweight, native, open-source multi-database GUI client.**

The speed and minimalism of TablePlus — open source. Many databases, one clean
UI. Every backend difference hides behind a single `Driver` trait, so the
interface stays small and identical no matter what you connect to. Built on
Tauri, not Electron: a tiny binary and low memory, using the system webview.

![License: MIT](https://img.shields.io/badge/license-MIT-blue)
![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)
![Svelte](https://img.shields.io/badge/Svelte-4-FF3E00?logo=svelte&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-sqlx-000000?logo=rust&logoColor=white)

> **Status:** active development. Postgres, MySQL/MariaDB and SQLite are fully
> usable for daily work. SQL Server and Redis/MongoDB are documented stubs.

<!-- Add a screenshot here: docs/screenshot.png -->

## Features

- **One UI for every database** — connect to PostgreSQL, MySQL/MariaDB and
  SQLite with the same screens. The UI never branches on database type.
- **Multi-connection workspaces** — keep several databases open in a left rail
  and switch instantly; each connection keeps its own tabs.
- **Schema browser** — schema switcher, table list with quick filter.
- **SQL editor** — CodeMirror with schema-aware context, per-tab, `⌘↵` to run.
  Table tabs (grid) and SQL query tabs (editor) are kept separate.
- **Fast result grid** — virtualized for large result sets.
- **Inline editing** — double-click a cell to edit; commit writes a precise
  `UPDATE` (primary-key aware, big-integer safe). Query results are editable too
  when the query is a simple single-table `SELECT *`.
- **Row detail panel** — view/edit a row field-by-field with type-aware controls:
  boolean dropdowns, JSON pretty/minify, and quick `NULL` / empty values.
- **Filter bar** — build `WHERE` conditions (`=`, `≠`, `>`, `contains`,
  `is null`, …) without writing SQL.
- **Table & column management** — create / rename / drop / truncate / duplicate
  tables, add / rename / drop columns, change type and nullability. All DDL is
  generated in the backend, per dialect.
- **Query log** — a toggleable panel showing every statement Kueri runs, with
  timing.
- **Command palette** (`⌘P`) — fuzzy-find and open any table across schemas.
- **Keyboard-first** — see the shortcuts below.
- **Safe credentials** — connections persist to disk; passwords go to the OS
  keychain, never plaintext.

## Database support

| Database | Status | Backend |
|----------|--------|---------|
| PostgreSQL | ✅ implemented | `sqlx` |
| MySQL / MariaDB | ✅ implemented | `sqlx` |
| SQLite | ✅ implemented | `sqlx` |
| SQL Server | ⛔ stub | needs `tiberius` |
| Redis | ⛔ stub | needs a non-tabular UI |
| MongoDB | ⛔ stub | needs a document UI |

Adding a relational database is one file: implement the `Driver` trait and add a
`DbKind` variant — no UI or command changes needed.

## Getting started

**Prerequisites:** [Node.js](https://nodejs.org) 18+, the
[Rust toolchain](https://rustup.rs), and the
[Tauri system dependencies](https://v2.tauri.app/start/prerequisites/) for your
OS.

```bash
git clone https://github.com/umarta/kueri.git
cd kueri
npm install
npm run tauri dev
```

The first build compiles Tauri and the three database drivers, so it's slow;
later runs are fast.

**Build a release bundle:**

```bash
npm run tauri build
```

## Keyboard shortcuts

| Shortcut | Action |
|----------|--------|
| `⌘P` | Open anything (search & open a table) |
| `⌘T` / `⌘E` | New tab / open SQL editor |
| `⌘W` | Close tab |
| `⌘[` / `⌘]` | Previous / next tab |
| `⌘1`–`⌘9` | Jump to tab N |
| `⌘↵` | Run query |
| `⌘S` | Commit edits |
| `⌘F` | Toggle filter bar |
| `Space` | Toggle row detail panel |
| `⌘⌃[` / `⌘⌃]` | Data / Structure view |
| `⌘K` | Switch schema |
| `⌘R` | Refresh |

## Architecture

```
Svelte component → api.* (src/lib/tauri.ts) → Tauri command
   → db::open(cfg) picks a driver by DbKind
   → Box<dyn Driver> (postgres / mysql / sqlite / …)
   → returns the same shapes (SchemaInfo / TableInfo / QueryResult)
```

The `Driver` trait (`src-tauri/src/db/driver.rs`) is the seam. Components and
Tauri commands are database-agnostic; all backend differences — including SQL
dialect for DDL — live behind the trait.

```
src/
  components/        Svelte UI (database-agnostic)
  lib/
    tauri.ts         typed wrappers over every Rust command
    stores/          connection, workspace and query-log state
    ddl.ts           UI helpers for the table designers
src-tauri/src/
  db/
    driver.rs        the Driver trait + shared result types
    mod.rs           DbKind enum + open() factory
    ddl.rs           dialect-aware DDL generation
    postgres.rs …    per-database drivers
  commands.rs        Tauri commands (database-agnostic)
```

## Roadmap

- Insert / duplicate / delete rows
- Sort by clicking a column header
- Show / hide and reorder columns
- SQL Server driver (`tiberius`)
- SSH tunnels
- NoSQL mode (Redis key browser, MongoDB document view)

## Contributing

Contributions are welcome — see [`CONTRIBUTING.md`](CONTRIBUTING.md). Bug reports
and feature requests via issues, code via pull requests.

## License

[MIT](LICENSE)
