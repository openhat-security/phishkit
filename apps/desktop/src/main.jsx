import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";

// WebdriverIO guest bridge — only pulled into test-hooks builds.
if (import.meta.env.VITE_TEST_HOOKS === "1") {
  import("@wdio/tauri-plugin").catch(() => {
    /* optional in production installs */
  });
}

ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
