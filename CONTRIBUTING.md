# Contributing to Kueri

Thanks for your interest! Kueri is small on purpose — contributions that keep it
**fast, simple, and Postgres-focused** are the most welcome.

## Setup
```bash
npm install
npm run tauri dev
```
You need the Rust toolchain (`rustup`) and a Postgres instance to test against.

## Before opening a PR
- `cargo fmt` and `cargo clippy` (Rust) — no warnings.
- `npm run check` (Svelte/TS type check) passes.
- Keep PRs focused; one feature/fix per PR.
- New Tauri commands: register in `lib.rs` and add a typed wrapper in
  `src/lib/tauri.ts` (see `CLAUDE.md` → "Adding a new Rust command").

## Scope
In scope: Postgres browsing/querying UX, performance, keyboard workflow.
Out of scope (for now): other databases, plugin systems, ORM features.
Open an issue to discuss anything large before building it.

## Reporting bugs
Include: OS + version, Postgres version, steps to reproduce, and the exact
error text from the app.
