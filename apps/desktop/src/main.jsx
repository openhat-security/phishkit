import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";

// WebdriverIO guest bridge — only pulled into e2e-featured builds.
if (import.meta.env.VITE_E2E === "1") {
  import("@wdio/tauri-plugin").catch(() => {
    /* optional in non-e2e installs */
  });
}

ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
