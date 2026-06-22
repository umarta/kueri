# Kueri

A lightweight, native multi-database client. Built with Tauri (Rust) + Svelte.

The goal is simple: the speed and minimalism of TablePlus, open source. Many
databases, one clean UI — differences are hidden behind a `Driver` trait so the
app stays small. No Electron bloat.

> Early WIP. Implemented: PostgreSQL, MySQL/MariaDB, SQLite (connect, browse
> schemas/tables, run SQL, view results). SQL Server and Redis/MongoDB are
> stubbed with integration plans in the code — see `CLAUDE.md`.

## Why
Most Postgres GUIs are either heavy (Electron/Java) or paid. Kueri stays small
by doing one thing well: fast, native Postgres browsing and querying.

## Stack
- **Tauri v2** — native shell, system webview (tiny binary, low RAM)
- **Rust + sqlx** — async Postgres access
- **Svelte 4 + TypeScript** — UI

## Develop
```bash
# prerequisites: Node 18+, Rust (rustup), a Postgres to connect to
npm install
npm run tauri dev
```
First compile is slow (Tauri + sqlx). Subsequent runs are fast.

To build a release bundle:
```bash
npm run tauri icon path/to/logo.png   # generate app icons (do this once)
npm run tauri build
```

## Status / roadmap
See `CLAUDE.md` for the architecture and the MVP roadmap. Next up: connection
persistence, Keychain password storage, CodeMirror editor, grid virtualization,
and inline edit-and-commit.

## Contributing
See `CONTRIBUTING.md`. Issues and PRs welcome.

## License
MIT — see `LICENSE`.
