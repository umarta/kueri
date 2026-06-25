<img src="docs/logo.png" alt="Kueri" width="96" />

# Kueri

**A fast, native, open-source multi-database GUI client.**

The speed and minimalism of TablePlus — open source. One calm, keyboard-first UI for every database, with all backend differences hidden behind a single Rust `Driver` trait. Built on Tauri, not Electron: a tiny binary and low memory using the system webview.

![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg) 

![Latest release](https://img.shields.io/github/v/release/umarta/kueri?label=download&color=0a84ff) 

![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white) 

![Svelte 4](https://img.shields.io/badge/Svelte-4-FF3E00?logo=svelte&logoColor=white) 

![Rust + sqlx](https://img.shields.io/badge/Rust-sqlx-000000?logo=rust&logoColor=white) 

![macOS · Windows · Linux](https://img.shields.io/badge/macOS%20%C2%B7%20Windows%20%C2%B7%20Linux-grey)[**Download**](https://github.com/umarta/kueri/releases/latest) · [Features](#features) · [Screenshots](#screenshots) · [Install](#install) · [Shortcuts](#keyboard-shortcuts) · [Architecture](#architecture) · [Contributing](#contributing)

<img src="docs/screenshots/02-grid.png" alt="Browsing a table in Kueri" width="900" />

> [!NOTE]
> Active development. PostgreSQL, MySQL/MariaDB and SQLite are fully usable for daily work. SQL Server, Redis and MongoDB are documented stubs.

## Why Kueri?

Most database GUIs are heavy (Electron/Java), closed-source, or paid. Kueri stays small and consistent: one keyboard-first interface, every database-specific detail pushed behind one Rust abstraction — so it behaves identically on Postgres, MySQL, or SQLite, and starts in a blink.

- **Tiny & native** — a Tauri binary on the system webview, not a bundled browser.
- **Keyboard-first** — run, format, commit, switch tabs, and open anything (`⌘P`) without touching the mouse.
- **One abstraction** — the UI never branches on database type; adding a driver is a single Rust file.
- **Safe by default** — passwords in the OS keychain, a read-only lock for production, precise primary-key-aware writes.

## Screenshots

| Browse & edit | Row detail — typed editors |
| :---: | :---: |
| <img src="docs/screenshots/02-grid.png" width="430" alt="Data grid" /> | <img src="docs/screenshots/03-row-detail.png" width="430" alt="Row detail panel" /> |
| **Query editor + results** | **Structure — columns & indexes** |
| <img src="docs/screenshots/05-query-editor.png" width="430" alt="SQL editor" /> | <img src="docs/screenshots/04-structure.png" width="430" alt="Structure view" /> |

## Features

### Connect

- **Multi-connection workspaces** — keep several databases open in a left rail and switch instantly; each keeps its own tabs. **Session restore** reopens them on launch.
- **Connect anywhere** — SSL/TLS options and an optional **SSH tunnel** per connection; colour-, tag- and group your connections.
- **Safe credentials** — connections persist to disk; passwords go to the OS keychain, never plaintext.

### Browse & edit

- **Fast result grid** — virtualized for large results: **multi-column sort** (Shift-click headers), **paginate**, **find-in-results**, **show/hide columns** (persisted per table), multi-row **select + copy**, **delete** and **duplicate** rows.
- **Inline & detail editing** — edit in the grid or a field-by-field **row-detail panel** with type-aware controls: boolean & enum dropdowns, **foreign-key lookup dropdowns**, JSON pretty/minify, quick `NULL`, and **date / time / timezone pickers** for temporal columns. Edits stay in sync across both surfaces.
- **Right-click everything** — context menus on cells/rows (copy as **CSV/JSON/Markdown/SQL**, bulk **Set NULL / Fill**, quick **Filter / Exclude**, delete), tables, connections and history.
- **Insert rows** — a type-aware form built from the table's columns (works on empty tables too); the pending row is shown live at the foot of the grid.
- **Filters & FK navigation** — build `WHERE` conditions without SQL; click a foreign-key cell to jump to the referenced row.
- **Safe writes** — commits are precise primary-key-aware `UPDATE`s; big integers keep full precision; a read-only lock disables editing entirely.

### Write SQL

- **CodeMirror editor** — schema-aware autocomplete, per-tab, `⌘↵` to run, **find & replace**, and **format** (`⇧⌘F`).
- **Multi-statement** scripts with a result-set switcher; **cancel** a long query (`⌘.`); **visual EXPLAIN** plan tree (Postgres); **generate SQL** (SELECT/INSERT/UPDATE/CREATE) from any table.
- **Transactions** — manual **Begin / Commit / Rollback** on a pinned connection; a guard confirms a console `UPDATE`/`DELETE` with no `WHERE`.
- **Saved queries** and a date-grouped, searchable **History** of console runs (with a right-click menu), kept separate from the **activity log** that records everything the app runs.

### Schema & structure

- **Three-tab sidebar** — Items (tables/views expandable to columns, plus **functions / triggers / sequences**), Queries, History.
- **Structure tab** — TablePlus-style two-pane columns + indexes grids (type / nullable / default / **foreign key** / **editable comment**); **manage indexes & foreign keys**, view the **CREATE / DDL**, and **edit a view's definition**.
- **Object & DDL management** — create / rename / drop / truncate / duplicate tables, add / rename / drop columns, and **create / drop database & schema** — all DDL generated per dialect in the backend.
- **Server tools** — a **Server Monitor** (running queries + kill) and a **users / roles** list.

### Import, export & backup

- **Native SQL export** — generated in-app over the connection, so it never hits a client/server version mismatch (no external tools required).
- **Backup & restore** — `pg_dump` / `pg_restore` custom format with **automatic matching-client detection & install**, MySQL `mysqldump`, and SQLite file copy.
- **CSV import** — into a table, with column mapping and a preview.

### Productivity

- **Command palette** (`⌘P`) to open any table; **read-only / safe mode** lock (auto-on for production-tagged connections).
- **Light / dark / auto** theme, native menu bar & Settings, keyboard-first throughout.

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

**Debian / Ubuntu (**`.deb`**) — recommended.** The package declares its runtime dependencies, so `apt` installs WebKitGTK and friends for you:

```bash
sudo apt install ./Kueri_<version>_amd64.deb
kueri
```

**Fedora / RHEL** (`.rpm`) — same, `dnf` resolves the deps:

```bash
sudo dnf install ./Kueri-<version>-1.x86_64.rpm
kueri
```

**AppImage** (portable). It does **not** resolve dependencies, so install the runtime WebKitGTK once if the window is blank or it won't start:

```bash
sudo apt install libwebkit2gtk-4.1-0          # runtime lib (note: NOT the -dev package)
chmod +x Kueri_<version>_amd64.AppImage
./Kueri_<version>_amd64.AppImage
```

> **Runtime vs build deps.** Running Kueri needs only the runtime lib `libwebkit2gtk-4.1-0` (the `.deb`/`.rpm` pull it in automatically). The longer `…-dev` list (`libwebkit2gtk-4.1-dev`, `build-essential`, `libssl-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, …) is only needed to **build from source**, not to run a release.
> ****Blank window / WebKitWebProcess crash?** Usually WebKitGTK's DMABUF renderer on certain GPU drivers. Kueri sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` itself, but if you still hit it, also try `WEBKIT_DISABLE_COMPOSITING_MODE=1 kueri`.

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

Every release is recorded in [CHANGELOG.md](CHANGELOG.md). Recent work brought Kueri close to TablePlus / Navicat parity — keyboard-driven grid, multi-statement scripts, the three-tab sidebar, manual transactions, server monitor, themes, native + matching-client backups, and type-aware date/time/timezone editors.

## Roadmap

- [x] Duplicate / delete rows · sort · show/hide columns

- [x] SSH tunnels · SSL/TLS

- [x] `EXPLAIN` · multi-statement · transactions · server monitor

- [x] Native SQL export + matching `pg_dump` auto-detect/install

- [ ] Visual query-plan viewer

- [ ] Browse functions / triggers / sequences

- [ ] Data transfer & structure sync between connections

- [ ] SQL Server driver (`tiberius`)

- [ ] NoSQL mode (Redis key browser, MongoDB document view)

See the [TablePlus / Navicat parity milestone](https://github.com/umarta/kueri/milestone/2) for the full backlog.

## Contributing

Contributions are welcome! Bug reports and feature requests via [issues](https://github.com/umarta/kueri/issues), code via pull requests.

README screenshots are generated from the built frontend with a mocked backend — see `scripts/capture-screenshots.mjs` (run with Playwright).

## License

[MIT](LICENSE) © Kueri contributors