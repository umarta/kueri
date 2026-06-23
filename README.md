<img src="docs/logo.png" alt="Kueri" width="120" />

# Kueri

**A lightweight, native, open-source multi-database GUI client.**

The speed and minimalism of TablePlus — open source. One clean UI for every database, with backend differences hidden behind a single `Driver` trait. Built on Tauri (not Electron): a tiny binary and low memory, using the system webview.

![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)![Svelte](https://img.shields.io/badge/Svelte-4-FF3E00?logo=svelte&logoColor=white)![Rust](https://img.shields.io/badge/Rust-sqlx-000000?logo=rust&logoColor=white)![Platform](https://img.shields.io/badge/macOS%20%C2%B7%20Windows%20%C2%B7%20Linux-grey)[Features](#features) · [Getting started](#getting-started) · [Shortcuts](#keyboard-shortcuts) · [Architecture](#architecture) · [Contributing](#contributing)

> [!NOTE] 💡
> Status: active development. PostgreSQL, MySQL/MariaDB and SQLite are fully usable for daily work. SQL Server and Redis/MongoDB are documented stubs.

## Why Kueri?

Most database GUIs are either heavy (Electron/Java), closed-source, or paid. Kueri keeps a single, calm, keyboard-first interface and pushes every database-specific detail behind one Rust abstraction — so the app stays small and behaves identically whether you're on Postgres, MySQL, or SQLite.

## Features

- **One UI for every database** — the interface never branches on database type.
- **Multi-connection workspaces** — keep several databases open in a left rail and switch instantly; each keeps its own tabs.
- **Schema browser** — schema switcher and a filterable table list.
- **SQL editor** — CodeMirror with schema-aware context, per tab, `⌘↵` to run. Table tabs (grid) and query tabs (editor) are separate.
- **Fast result grid** — virtualized for large result sets, with inline cell editing, **sort** by header, **pagination**, **find-in-results**, **show/hide columns** (persisted per table), multi-row **select + copy**, **delete** and **duplicate** rows.
- **Safe edits** — commits write a precise primary-key-aware `UPDATE`; big integers keep full precision. Query results are editable when the query is a simple single-table `SELECT *`.
- **Insert rows** — a type-aware form built from the table's columns (works on empty tables too); empty fields fall back to column defaults. **Enum columns** edit via a dropdown.
- **Row detail panel** — edit a row field-by-field with type-aware controls (boolean & enum dropdowns, JSON pretty/minify, quick `NULL`/empty).
- **Filters** — build `WHERE` conditions without writing SQL.
- **Structure tab** — TablePlus-style two-pane view: a columns grid (type / nullable / default / **foreign key** / comment) and an indexes grid; **manage indexes & foreign keys**, view the **CREATE / DDL**.
- **Foreign-key navigation** — click an FK cell to jump to the referenced row.
- **Table & column management** — create / rename / drop / truncate / duplicate tables and add / rename / drop columns; all DDL is generated per dialect in the backend.
- **Import & export** — import CSV into a table (column mapping + preview) and export results to CSV / JSON; PostgreSQL backup & restore (`pg_dump` / `pg_restore`).
- **Read-only / safe mode** — a per-connection lock that blocks writes & DDL (defaults on for production-tagged connections).
- **Query history** — persistent, searchable, click-to-load; cancel a running query with `⌘.`.
- **SQL formatter** (`⇧⌘F`), **command palette** (`⌘P`) to open any table.
- **Connect anywhere** — SSL/TLS options and an optional **SSH tunnel** per connection.
- **Native menu bar & Settings**, keyboard-first throughout.
- **Safe credentials** — connections persist to disk; passwords go to the OS keychain, never plaintext.

## Supported databases

| Database | Status | Backend |
| --- | --- | --- |
| PostgreSQL | ✅ Implemented | `sqlx` |
| MySQL / MariaDB | ✅ Implemented | `sqlx` |
| SQLite | ✅ Implemented | `sqlx` |
| SQL Server | ⛔ Stub | needs `tiberius` |
| Redis | ⛔ Stub | non-tabular UI |
| MongoDB | ⛔ Stub | document UI |

> Adding a relational database is one file: implement the `Driver` trait and add a `DbKind` variant — no UI or command changes required.

## Install

Grab the installer for your platform from the [**latest release**](https://github.com/umarta/kueri/releases/latest).

> Builds are currently **unsigned** (normal for an open-source project), so each OS asks for confirmation on first launch — steps below.

### macOS

1. Download `Kueri_<version>_aarch64.dmg` (Apple Silicon) or `Kueri_<version>_x64.dmg` (Intel).

2. Open the `.dmg` and drag **Kueri** into **Applications**.

3. First launch: **right-click** `Kueri.app` **→ Open → Open** (Gatekeeper only blocks a double-click). If it still refuses:

   ```bash
   xattr -dr com.apple.quarantine /Applications/Kueri.app
   ```

### Windows

1. Download `Kueri_<version>_x64-setup.exe` (or the `.msi`).
2. Run it. If **SmartScreen** appears, click **More info → Run anyway**.
3. Launch **Kueri** from the Start menu.

### Linux

**AppImage** (portable, works on most distros):

```bash
chmod +x Kueri_<version>_amd64.AppImage
./Kueri_<version>_amd64.AppImage
```

**Debian / Ubuntu** (`.deb`):

```bash
sudo apt install ./Kueri_<version>_amd64.deb
kueri
```

**Fedora / RHEL** (`.rpm`):

```bash
sudo dnf install ./Kueri-<version>-1.x86_64.rpm
kueri
```

> Kueri uses WebKitGTK (`libwebkit2gtk-4.1`). The `.deb`/`.rpm` pull it in automatically; for the AppImage, install it first if the app won't start (e.g. `sudo apt install libwebkit2gtk-4.1-0`).

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

| Shortcut | Action |
| --- | --- |
| `⌘P` | Open anything (search a table) |
| `⌘T` / `⌘E` | New tab / new SQL editor |
| `⌘W` | Close tab |
| `⌘[` / `⌘]` | Previous / next tab |
| `⌘1`–`⌘9` | Jump to tab |
| `⌘N` | New connection |
| `⌘K` | Switch schema |
| `⌘R` | Reload workspace |
| `⌘↵` | Run query |
| `⌘.` | Cancel running query |
| `⇧⌘F` | Format SQL |
| `⌘S` | Commit changes |
| `⌘I` | Add row |
| `⌘D` | Duplicate row |
| `⌘F` | Toggle filters |
| `Space` | Toggle row detail |
| `⌘⌃[` / `⌘⌃]` | Data / Structure view |
| `⌘,` | Settings |

## Architecture

```text
Svelte component → api.* (src/lib/tauri.ts) → Tauri command
   → db::open(cfg) picks a driver by DbKind
   → Box<dyn Driver> (postgres / mysql / sqlite / …)
   → returns the same shapes (SchemaInfo / TableInfo / QueryResult)
```

The `Driver` trait (`src-tauri/src/db/driver.rs`) is the seam. Components and Tauri commands are database-agnostic; all backend differences — including SQL dialect for DDL — live behind the trait.

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

## Changelog

See [CHANGELOG.md](CHANGELOG.md). The latest release, **v0.2.0**, completes the data grid (delete / sort / paginate / find / copy / export / import) and adds a TablePlus-style Structure tab, foreign-key navigation, read-only mode, query history, SSL & SSH options.

## Roadmap

- [x] Duplicate / delete rows · sort · show/hide columns — shipped in v0.2

- [x] SSH tunnels — shipped in v0.2

- [ ] Column reordering & resize

- [ ] `EXPLAIN` / query plan viewer

- [ ] SQL Server driver (`tiberius`)

- [ ] NoSQL mode (Redis key browser, MongoDB document view)

## Contributing

Contributions are welcome! Please read `CONTRIBUTING.md` — bug reports and feature requests via [issues](https://github.com/umarta/kueri/issues), code via pull requests.

## License

[MIT](LICENSE) © Kueri contributors