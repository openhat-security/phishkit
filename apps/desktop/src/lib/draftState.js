// Lightweight per-view draft persistence so in-progress campaign state survives
// tab switches (which unmount the view) and app reloads. Values are namespaced
// and stored as a single JSON blob per key; failures (private mode, quota,
// missing localStorage) degrade silently to in-memory-only behavior.

const PREFIX = "phishkit.draft.";

export function loadDraft(key) {
  try {
    const raw = localStorage.getItem(PREFIX + key);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

export function saveDraft(key, obj) {
  try {
    localStorage.setItem(PREFIX + key, JSON.stringify(obj));
  } catch {
    /* ignore: storage unavailable or over quota */
  }
}

export function clearDraft(key) {
  try {
    localStorage.removeItem(PREFIX + key);
  } catch {
    /* ignore */
  }
}
