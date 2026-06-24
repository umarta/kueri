// Capture Kueri screenshots for the README: serve the built frontend, mock the
// Tauri invoke bridge with demo data, drive the UI, screenshot key views.
import { chromium } from "playwright";
import { mkdirSync } from "fs";

const BASE = process.env.KUERI_URL || "http://localhost:4173";
const OUT = "/Users/umarta/Works/own/kueri/kueri/docs/screenshots";
mkdirSync(OUT, { recursive: true });

// ── Demo dataset (an e-commerce schema), serialized into the page ────────────
const DATA = {
  connections: [{
    id: "demo", name: "Acme Production", kind: "postgres",
    host: "db.acme.internal", port: 5432, user: "acme", database: "shop",
    file_path: null, color: "green", tag: "prod", group: "Acme",
    ssl_mode: "", ssl_ca: "", ssl_cert: "", ssl_key: "",
    ssh_enabled: false, ssh_host: "", ssh_port: 22, ssh_user: "", ssh_key: "",
  }],
  schemas: [{ name: "public" }, { name: "analytics" }],
  tables: [
    { name: "orders", kind: "BASE TABLE" }, { name: "customers", kind: "BASE TABLE" },
    { name: "products", kind: "BASE TABLE" }, { name: "order_items", kind: "BASE TABLE" },
    { name: "categories", kind: "BASE TABLE" }, { name: "payments", kind: "BASE TABLE" },
    { name: "inventory", kind: "BASE TABLE" }, { name: "shipments", kind: "BASE TABLE" },
    { name: "active_orders", kind: "VIEW" }, { name: "revenue_by_day", kind: "VIEW" },
  ],
  columns: [
    { name: "id", data_type: "bigint", nullable: false, default: "nextval(...)", enum_values: [], comment: "Primary key" },
    { name: "customer_id", data_type: "bigint", nullable: false, default: null, enum_values: [], comment: "FK -> customers.id" },
    { name: "status", data_type: "order_status", nullable: false, default: "'pending'", enum_values: ["pending", "paid", "shipped", "delivered", "cancelled"], comment: null },
    { name: "total", data_type: "numeric", nullable: false, default: "0", enum_values: [], comment: "Order total (USD)" },
    { name: "currency", data_type: "character varying", nullable: false, default: "'USD'", enum_values: [], comment: null },
    { name: "placed_at", data_type: "timestamp with time zone", nullable: false, default: "now()", enum_values: [], comment: null },
    { name: "notes", data_type: "text", nullable: true, default: null, enum_values: [], comment: null },
  ],
  pkeys: ["id"],
  fkeys: [{ column: "customer_id", ref_schema: "public", ref_table: "customers", ref_column: "id" }],
  indexes: [
    { name: "orders_pkey", columns: ["id"], unique: true, method: "btree", predicate: "" },
    { name: "orders_customer_idx", columns: ["customer_id"], unique: false, method: "btree", predicate: "" },
    { name: "orders_status_idx", columns: ["status"], unique: false, method: "btree", predicate: "status <> 'cancelled'" },
  ],
  ddl: "CREATE TABLE public.orders (\n  id bigint NOT NULL DEFAULT nextval('orders_id_seq'),\n  customer_id bigint NOT NULL,\n  status order_status NOT NULL DEFAULT 'pending',\n  total numeric NOT NULL DEFAULT 0,\n  currency varchar NOT NULL DEFAULT 'USD',\n  placed_at timestamptz NOT NULL DEFAULT now(),\n  notes text,\n  PRIMARY KEY (id)\n);",
  rows: [
    [10241, 5012, "delivered", 129.0, "USD", "2026-06-21 09:14:02+00", "Gift wrap"],
    [10242, 4820, "shipped", 58.5, "USD", "2026-06-21 10:02:55+00", null],
    [10243, 5113, "paid", 412.2, "USD", "2026-06-21 11:31:40+00", null],
    [10244, 4901, "pending", 24.0, "USD", "2026-06-22 08:05:11+00", "Call before delivery"],
    [10245, 5240, "delivered", 76.9, "USD", "2026-06-22 12:48:19+00", null],
    [10246, 5012, "cancelled", 199.0, "USD", "2026-06-22 14:20:03+00", "Customer changed mind"],
    [10247, 4777, "paid", 340.75, "USD", "2026-06-23 07:59:48+00", null],
    [10248, 5301, "shipped", 18.25, "USD", "2026-06-23 09:11:27+00", null],
    [10249, 4820, "delivered", 510.0, "USD", "2026-06-23 16:42:10+00", "VIP"],
    [10250, 5188, "pending", 64.4, "USD", "2026-06-24 06:33:02+00", null],
  ],
};

function initScript(dataJson, seed) {
  return `
    window.__TAURI_INTERNALS__ = {
      transformCallback: (cb) => cb,
      invoke: (cmd, args) => {
        const D = ${dataJson};
        const map = {
          load_connections: D.connections,
          secret_get: "",
          connect: "demo",
          list_schemas: D.schemas,
          list_tables: D.tables,
          list_columns: D.columns,
          primary_keys: D.pkeys,
          foreign_keys: D.fkeys,
          list_indexes: D.indexes,
          table_ddl: D.ddl,
          execute_query: { columns: D.columns.map((c) => c.name), rows: D.rows, row_count: D.rows.length },
          list_processes: [],
          list_roles: [],
        };
        return Promise.resolve(cmd in map ? map[cmd] : null);
      },
    };
    localStorage.setItem("kueri.settings", JSON.stringify({ rowLimit: 200, altRows: true, theme: "dark", toolsPath: "", showSidebarRowDetail: true }));
    ${seed ? `localStorage.setItem("kueri.session", JSON.stringify({ open: ["demo"], active: "demo" }));` : `localStorage.removeItem("kueri.session");`}
  `;
}

const shot = (page, name) => page.screenshot({ path: `${OUT}/${name}.png` });
const dataJson = JSON.stringify(DATA);

const run = async () => {
  const browser = await chromium.launch();
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2 });

  const w = await ctx.newPage();
  await w.addInitScript(initScript(dataJson, false));
  await w.goto(BASE, { waitUntil: "networkidle" });
  await w.waitForTimeout(1500);
  await shot(w, "01-connections");
  await w.close();

  const p = await ctx.newPage();
  await p.addInitScript(initScript(dataJson, true));
  await p.goto(BASE, { waitUntil: "networkidle" });
  await p.waitForTimeout(1800);
  try {
    await p.getByText("orders", { exact: true }).first().click({ timeout: 6000 });
    await p.waitForTimeout(1500);
  } catch (e) { console.log("table click:", e.message); }
  await shot(p, "02-grid");

  // Row-detail panel — click a data row.
  try {
    await p.getByText("10243", { exact: true }).first().click({ timeout: 4000 });
    await p.waitForTimeout(1000);
    await shot(p, "03-row-detail");
  } catch (e) { console.log("detail:", e.message); }

  // Structure tab.
  try {
    await p.getByText("Structure", { exact: true }).first().click({ timeout: 4000 });
    await p.waitForTimeout(1200);
    await shot(p, "04-structure");
  } catch (e) { console.log("structure:", e.message); }

  // Query editor with results.
  try {
    await p.getByText("Query 1", { exact: true }).first().click({ timeout: 4000 });
    await p.waitForTimeout(600);
    const cm = p.locator(".cm-content").first();
    await cm.click({ timeout: 4000 });
    await p.keyboard.press("Meta+a");
    await p.keyboard.press("Backspace");
    await p.keyboard.type("SELECT id, status, total, placed_at\nFROM orders\nWHERE status = 'paid'\nORDER BY placed_at DESC;");
    await p.waitForTimeout(400);
    await p.keyboard.press("Meta+Enter");
    await p.waitForTimeout(1200);
    await shot(p, "05-query-editor");
  } catch (e) { console.log("query:", e.message); }

  await browser.close();
  console.log("done →", OUT);
};
run().catch((e) => { console.error(e); process.exit(1); });
