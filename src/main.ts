import App from "./App.svelte";
import "./app.css";

// In production builds, prevent opening developer tools / "Inspect Element".
// Release builds already ship without Tauri's `devtools` feature (so WKWebView's
// inspector is not compiled in); this additionally removes the right-click
// context menu and blocks the devtools / view-source keyboard shortcuts.
// Left enabled in `tauri dev` so the inspector is still available while developing.
if (import.meta.env.PROD) {
  window.addEventListener("contextmenu", (e) => e.preventDefault());
  window.addEventListener("keydown", (e) => {
    const k = e.key.toLowerCase();
    const mod = e.metaKey || e.ctrlKey;
    if (
      k === "f12" || // devtools
      (mod && (e.altKey || e.shiftKey) && (k === "i" || k === "j" || k === "c")) || // inspect / console
      (mod && k === "u") // view source
    ) {
      e.preventDefault();
    }
  });
}

const app = new App({ target: document.getElementById("app")! });
export default app;
