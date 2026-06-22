# Product

## Register

product

## Users
Developers, data engineers, and DBAs who work across PostgreSQL, MySQL/MariaDB,
and SQLite (more to come). They live in this app for hours: browsing schemas,
writing SQL, scanning result grids, editing rows. They are fluent in tools like
TablePlus, DataGrip, and Postico, and expect that level of speed and polish.
Context: a native desktop window on macOS, often alongside an editor and a
terminal, frequently switching between local, staging, and production databases.

## Product Purpose
Kueri is a lightweight, native, open-source multi-database GUI client. It hides
the complexity of many databases behind one abstraction (the `Driver` trait) so
the UI stays simple and identical regardless of backend. Success = a user trusts
it for daily work and never thinks about the tool — only the data. The benchmark
is TablePlus-grade simplicity and speed, but open source.

## Brand Personality
Calm, precise, native, fast. Three words: **quiet, sharp, trustworthy.** The
interface should feel like a well-made instrument — no chrome for its own sake,
no decoration competing with the data. It earns trust by being predictable and
keyboard-first (⌘↵ runs SQL). Voice in copy is terse and technical, never cute.

## Anti-references
- **DBeaver / pgAdmin** — dense Java-era toolbars, ramped icons, heavy nested
  panels, dated feel. Avoid clutter and toolbar overload.
- **Enterprise / corporate admin dashboards** — card grids, gradients, hero
  metrics, excess chrome. This is a tool, not a dashboard.
- **Playful / toy aesthetics** — bright candy colors, oversized radii, cute
  illustrations. This is serious work software.
- **Generic web app** — must not feel like "a website inside a window." It
  should read as a native macOS application.

## Design Principles
1. **The tool disappears into the task.** Data is the hero; UI is the frame.
   Earned familiarity over novelty — standard affordances, no invented controls.
2. **Database-agnostic UI.** The interface never branches on DB type. One visual
   vocabulary for Postgres, MySQL, SQLite, and everything added later.
3. **Keyboard-first.** Every primary action has a shortcut; mouse is optional.
4. **Density with calm.** Show a lot (schemas, columns, many rows) without
   noise — restraint in color and ornament makes density legible.
5. **Color carries meaning, not mood.** Accent = selection and primary action.
   Saturated color is reserved for connection environment tags (local / staging /
   production) so prod is never mistaken for local.

## Accessibility & Inclusion
- Body text ≥ 4.5:1 contrast on its surface; selected/focus states clearly
  visible (not color-only — pair with weight/background).
- Full keyboard operability; visible focus rings.
- Honor `prefers-reduced-motion` — transitions convey state (150–250ms), never
  decoration, and degrade to instant/crossfade.
- Connection environment status is never conveyed by color alone (color dot +
  text label).
