<div align="center">

<img src="docs/logo.png" alt="Kueri" width="120" height="120" />

<h1>Kueri</h1>

**A lightweight, native, open-source multi-database GUI client.**

The speed and minimalism of TablePlus — open source.
One clean UI for every database, with backend differences hidden behind a single `Driver` trait.
Built on Tauri (not Electron): a tiny binary and low memory, using the system webview.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)
![Svelte](https://img.shields.io/badge/Svelte-4-FF3E00?logo=svelte&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-sqlx-000000?logo=rust&logoColor=white)
![Platform](https://img.shields.io/badge/macOS%20·%20Windows%20·%20Linux-grey)

[Features](#features) · [Getting started](#getting-started) · [Shortcuts](#keyboard-shortcuts) · [Architecture](#architecture) · [Contributing](#contributing)

</div>

> [!NOTE]
> **Status:** active development. PostgreSQL, MySQL/MariaDB and SQLite are fully usable for daily work. SQL Server and Redis/MongoDB are documented stubs.

<!-- Replace with a real screenshot once captured: docs/screenshot.png -->
<!-- <p align="center"><img src="docs/screenshot.png" alt="Kueri screenshot" width="900" /></p> -->

## Why Kueri?

Most database GUIs are either heavy (Electron/Java), closed-source, or paid. Kueri keeps a single, calm,
keyboard-first interface and pushes every database-specific detail behind one Rust abstraction — so the app
stays small and behaves identically whether you're on Postgres, MySQL, or SQLite.

## Features

- **One UI for every database** — the interface never branches on database type.
- **Multi-connection workspaces** — keep several databases open in a left rail and switch instantly; each keeps its own tabs.
- **Schema browser** — schema switcher and a filterable table list.
- **SQL editor** — CodeMirror with schema-aware context, per tab, `⌘↵` to run. Table tabs (grid) and query tabs (editor) are separate.
- **Fast result grid** — virtualized for large result sets, with inline cell editing.
- **Safe edits** — commits write a precise primary-key-aware `UPDATE`; big integers keep full precision. Query results are editable when the query is a simple single-table `SELECT *`.
- **Insert rows** — a type-aware form built from the table's columns (works on empty tables too); empty fields fall back to column defaults.
- **Row detail panel** — edit a row field-by-field with type-aware controls (boolean dropdowns, JSON pretty/minify, quick `NULL`/empty).
- **Filters** — build `WHERE` conditions without writing SQL.
- **Table & column management** — create / rename / drop / truncate / duplicate tables and add / rename / drop columns; all DDL is generated per dialect in the backend.
- **Query log** — a toggleable panel showing every statement Kueri runs, with timing.
- **Command palette** (`⌘P`) — fuzzy-find and open any table across schemas.
- **Native menu bar & Settings**, keyboard-first throughout.
- **Safe credentials** — connections persist to disk; passwords go to the OS keychain, never plaintext.

## Supported databases

| Database         | Status         | Backend            |
| ---------------- | -------------- | ------------------ |
| PostgreSQL       | ✅ Implemented | `sqlx`             |
| MySQL / MariaDB  | ✅ Implemented | `sqlx`             |
| SQLite           | ✅ Implemented | `sqlx`             |
| SQL Server       | ⛔ Stub        | needs `tiberius`   |
| Redis            | ⛔ Stub        | non-tabular UI     |
| MongoDB          | ⛔ Stub        | document UI        |

> Adding a relational database is one file: implement the `Driver` trait and add a `DbKind` variant — no UI or command changes required.

## Getting started

### Prerequisites

- [Node.js](https://nodejs.org) 18+
- [Rust](https://rustup.rs) (stable)
- [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/) for your OS

### Run from source

```bash
git clone https://github.com/umarta/kueri.git
cd kueri
npm install
npm run tauri dev
```

> The first build compiles Tauri and the three database drivers, so it's slow. Subsequent runs are fast.

### Build a release bundle

```bash
npm run tauri build
```

## Keyboard shortcuts

| Shortcut       | Action                          |
| -------------- | ------------------------------- |
| `⌘P`           | Open anything (search a table)  |
| `⌘T` / `⌘E`    | New tab / new SQL editor        |
| `⌘W`           | Close tab                       |
| `⌘[` / `⌘]`    | Previous / next tab             |
| `⌘1`–`⌘9`      | Jump to tab                     |
| `⌘N`           | New connection                  |
| `⌘K`           | Switch schema                   |
| `⌘R`           | Reload workspace                |
| `⌘↵`           | Run query                       |
| `⌘S`           | Commit changes                  |
| `⌘I`           | Add row                         |
| `⌘F`           | Toggle filters                  |
| `Space`        | Toggle row detail               |
| `⌘⌃[` / `⌘⌃]`  | Data / Structure view           |
| `⌘,`           | Settings                        |

## Architecture

```text
Svelte component → api.* (src/lib/tauri.ts) → Tauri command
   → db::open(cfg) picks a driver by DbKind
   → Box<dyn Driver> (postgres / mysql / sqlite / …)
   → returns the same shapes (SchemaInfo / TableInfo / QueryResult)
```

The `Driver` trait (`src-tauri/src/db/driver.rs`) is the seam. Components and Tauri commands are
database-agnostic; all backend differences — including SQL dialect for DDL — live behind the trait.

```text
src/
  components/        Svelte UI (database-agnostic)
  lib/
    tauri.ts         typed wrappers over every Rust command
    stores/          connection, workspace, settings, query-log state
    ddl.ts           UI helpers for the table designers
src-tauri/src/
  db/
    driver.rs        the Driver trait + shared result types
    mod.rs           DbKind enum + open() factory
    ddl.rs           dialect-aware DDL generation
    postgres.rs …    per-database drivers
  commands.rs        Tauri commands (database-agnostic)
  menu.rs            native application menu
```

### Adding a new relational driver

1. Create `src-tauri/src/db/<name>.rs` and `impl Driver` for it (`async_trait`).
2. Add a variant to `DbKind` and wire it in `db::mod.rs::open()`.
3. Add the variant to `DbKind` in `src/lib/types.ts` and the connection picker.

No UI or command changes are needed.

## Roadmap

- [ ] Duplicate / delete rows
- [ ] Sort by clicking a column header
- [ ] Show / hide and reorder columns
- [ ] SQL Server driver (`tiberius`)
- [ ] SSH tunnels
- [ ] NoSQL mode (Redis key browser, MongoDB document view)

## Contributing

Contributions are welcome! Please read [`CONTRIBUTING.md`](CONTRIBUTING.md) — bug reports and feature
requests via [issues](https://github.com/umarta/kueri/issues), code via pull requests.

## License

[MIT](LICENSE) © Kueri contributors
