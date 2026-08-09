import { invoke } from "@tauri-apps/api/core";

/**
 * Save text via the native save dialog.
 * WKWebView ignores `<a download>` / blob URLs, so browser-style downloads no-op on macOS.
 * @returns {Promise<string|null>} saved path, or null if cancelled / unavailable
 */
export async function downloadText(filename, text, _mime = "text/plain;charset=utf-8") {
  const name = String(filename || "download.txt").replace(/[/\\]/g, "-");
  const contents = typeof text === "string" ? text : String(text ?? "");
  try {
    return await invoke("save_text_download", {
      defaultName: name,
      contents,
    });
  } catch (e) {
    // Fallback for vite-only browser preview (no Tauri runtime).
    try {
      const blob = new Blob([contents], { type: _mime });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = name;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      setTimeout(() => URL.revokeObjectURL(url), 1000);
      return name;
    } catch {
      throw e;
    }
  }
}
