import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { fmtProfile } from "./lib/workspace";
import { downloadText } from "./lib/download";
import Hint from "./components/Hint";

export default function AssessmentOverview({
  assessment,
  status,
  append,
  busy,
  setBusy,
  onGoTargets,
  onGoCampaigns,
  onGoResults,
  onOpenTarget,
  onArchived,
  onRestored,
  onDeleted,
  onCloned,
}) {
  const [targets, setTargets] = useState([]);
  const [campaigns, setCampaigns] = useState([]);
  const [redactExport, setRedactExport] = useState(true);
  const [purgeSessions, setPurgeSessions] = useState(true);
  const [purgeAttempts, setPurgeAttempts] = useState(false);
  const [purgePii, setPurgePii] = useState(false);
  const [cleanupHosts, setCleanupHosts] = useState(true);

  const load = useCallback(async () => {
    if (!assessment?.id) return;
    try {
      const [t, c] = await Promise.all([
        invoke("cmd_list_targets", { assessmentId: assessment.id }),
        invoke("list_campaigns", { assessmentId: assessment.id }),
      ]);
      setTargets(t.map(fmtProfile));
      setCampaigns(c);
    } catch (e) {
      append(String(e));
    }
  }, [assessment?.id, append]);

  useEffect(() => {
    load();
    const t = setInterval(load, 8000);
    return () => clearInterval(t);
  }, [load]);

  const proxyUp = !!status?.evilginx_running;
  const withPhishlet = targets.filter((t) => t.phishlet).length;
  const withLure = targets.filter((t) => t.lureUrl).length;
  const recent = campaigns.slice(0, 5);

  const exportBundle = async () => {
    if (!assessment?.id) return;
    setBusy?.("export");
    try {
      const bundle = await invoke("cmd_export_assessment_bundle", {
        id: assessment.id,
        redact: redactExport,
      });
      const stamp = new Date().toISOString().slice(0, 10);
      const slug = (assessment.name || "assessment").replace(/[^a-z0-9]+/gi, "-");
      const path = await downloadText(
        `${slug}-${stamp}${redactExport ? "-redacted" : ""}.json`,
        JSON.stringify(bundle, null, 2),
        "application/json"
      );
      if (!path) {
        append("Export cancelled");
        return;
      }
      const c = bundle.counts || {};
      append(
        `Exported bundle · ${c.targets ?? 0} targets · ${c.campaigns ?? 0} campaigns · ${
          c.sessions ?? 0
        } sessions${redactExport ? " (redacted)" : ""} → ${path}`
      );
    } catch (e) {
      append(String(e));
    } finally {
      setBusy?.("");
    }
  };

  const runPurge = async () => {
    if (!assessment?.id) return;
    if (!purgeSessions && !purgeAttempts && !purgePii) {
      append("Select at least one data type to purge");
      return;
    }
    const parts = [
      purgeSessions && "captured sessions",
      purgeAttempts && "send attempts",
      purgePii && "recipient PII",
    ].filter(Boolean);
    const ok = window.confirm(
      `Permanently delete ${parts.join(", ")} for "${assessment.name}"?\n\n` +
        "Targets, phishlets, lures, and templates are kept. Export a bundle first if you need evidence."
    );
    if (!ok) return;
    setBusy?.("purge");
    try {
      const r = await invoke("cmd_purge_assessment_data", {
        id: assessment.id,
        sessions: purgeSessions,
        attempts: purgeAttempts,
        pii: purgePii,
      });
      append(
        `Purged · ${r.sessionsDeleted} sessions · ${r.attemptsDeleted} attempts · ` +
          `${r.recipientsDeleted} recipients · ${r.listsDeleted} lists removed`
      );
      await load();
    } catch (e) {
      append(String(e));
    } finally {
      setBusy?.("");
    }
  };

  const runHostsCleanup = async () => {
    if (!assessment?.id) return;
    setBusy?.("hosts");
    try {
      const r = await invoke("cmd_assessment_hosts_cleanup", { id: assessment.id });
      if (r.ok) {
        append(
          r.removed > 0
            ? `Removed ${r.removed} /etc/hosts entr${r.removed === 1 ? "y" : "ies"}`
            : "No phishkit /etc/hosts entries to remove"
        );
      } else if (r.manual) {
        append(`Manual /etc/hosts cleanup needed: ${(r.fqdns || []).join(", ")}`);
      } else {
        append(`Hosts cleanup: ${r.stderr || "cancelled"}`);
      }
    } catch (e) {
      append(String(e));
    } finally {
      setBusy?.("");
    }
  };

  const isArchived = (assessment?.status || "").toLowerCase() === "archived";

  const archiveAssessment = async () => {
    if (!assessment?.id) return;
    const ok = window.confirm(
      `Archive assessment "${assessment.name}"?\n\n` +
        `Archive does NOT delete data — it marks the engagement inactive. ` +
        `Find it under Assessments → Show archived to Restore, New from archive, or Delete.`
    );
    if (!ok) return;
    setBusy?.("archive");
    try {
      if (proxyUp) {
        try {
          await invoke("evilginx_stop");
        } catch {
          /* best effort */
        }
      }
      if (cleanupHosts) {
        try {
          await invoke("cmd_assessment_hosts_cleanup", { id: assessment.id });
        } catch {
          /* best effort */
        }
      }
      const archived = await invoke("cmd_archive_assessment", { id: assessment.id });
      append(`Archived assessment "${archived.name || assessment.name}"`);
      onArchived?.(archived);
    } catch (e) {
      append(String(e));
    } finally {
      setBusy?.("");
    }
  };

  const restoreAssessment = async () => {
    if (!assessment?.id) return;
    setBusy?.("unarchive");
    try {
      const restored = await invoke("cmd_unarchive_assessment", {
        id: assessment.id,
      });
      await invoke("cmd_set_active_assessment", { id: assessment.id });
      append(`Restored assessment "${restored.name || assessment.name}"`);
      onRestored?.(restored);
    } catch (e) {
      append(String(e));
    } finally {
      setBusy?.("");
    }
  };

  const cloneAssessment = async () => {
    if (!assessment?.id) return;
    setBusy?.("clone");
    try {
      const created = await invoke("cmd_clone_assessment", { id: assessment.id });
      await invoke("cmd_set_active_assessment", { id: created.id });
      append(
        `Created “${created.name}” from ${assessment.name} (Targets copied; campaigns/sessions not copied)`
      );
      onCloned?.(created);
    } catch (e) {
      append(String(e));
    } finally {
      setBusy?.("");
    }
  };

  const deleteAssessment = async () => {
    if (!assessment?.id) return;
    const ok = window.confirm(
      `DELETE assessment "${assessment.name}" permanently?\n\n` +
        `This removes Targets, Lures, Campaigns, Sessions, templates, and recipient lists ` +
        `for this engagement from the app database. It cannot be undone.\n\n` +
        `Shared phishlet YAML under kit/evilginx/phishlets/ is not deleted.`
    );
    if (!ok) return;
    const ok2 = window.confirm(
      `Type-confirm: permanently delete “${assessment.name}”? There is no recycle bin.`
    );
    if (!ok2) return;
    setBusy?.("delete");
    try {
      if (proxyUp) {
        try {
          await invoke("evilginx_stop");
        } catch {
          /* best effort */
        }
      }
      if (cleanupHosts) {
        try {
          await invoke("cmd_assessment_hosts_cleanup", { id: assessment.id });
        } catch {
          /* best effort */
        }
      }
      const r = await invoke("cmd_delete_assessment", { id: assessment.id });
      append(
        `Deleted assessment “${r.name}” · ${r.profilesDeleted ?? 0} targets · ` +
          `${r.campaignsDeleted ?? 0} campaigns`
      );
      onDeleted?.(r);
    } catch (e) {
      append(String(e));
    } finally {
      setBusy?.("");
    }
  };

  const actions = [
    {
      title: "Add Target",
      sub: `${targets.length} configured · ${withPhishlet} with Phishlet`,
      cta: "Targets",
      done: targets.length > 0,
      onClick: onGoTargets,
    },
    {
      title: "Start AiTM proxy",
      sub: proxyUp ? "Proxy live — open a Target to manage Lures" : "Proxy idle on Destinations",
      cta: "Open Target",
      done: proxyUp,
      onClick: () => {
        const pick = targets.find((t) => t.lureUrl) || targets[0];
        if (pick) onOpenTarget?.(pick);
        else onGoTargets();
      },
    },
    {
      title: "Launch Campaign",
      sub: withLure
        ? `${withLure} Target${withLure === 1 ? "" : "s"} with Lure URL`
        : "Generate a Lure on a Target first",
      cta: "Campaigns",
      done: campaigns.some((c) => c.status === "running" || c.sent > 0),
      onClick: onGoCampaigns,
    },
  ];

  return (
    <div className="assessment-overview" data-testid="overview-view">
      {isArchived && (
        <div className="info-banner" data-testid="overview-archived-banner">
          This assessment is archived (inactive in the database). Restore it, create New from
          archive, or permanently Delete below. Archive alone never erases data.
        </div>
      )}
      <div className="overview-stats">
        <div className="stat-card card">
          <span className="num">{targets.length}</span>
          <span className="lbl">Targets</span>
        </div>
        <div className="stat-card card">
          <span className={`dot ${proxyUp ? "up" : "down"}`} />
          <span className="lbl">{proxyUp ? "AiTM proxy live" : "Proxy idle"}</span>
        </div>
        <div className="stat-card card">
          <span className="num">{campaigns.length}</span>
          <span className="lbl">Campaigns</span>
        </div>
      </div>

      <section className="card">
        <h3>Next actions</h3>
        <ul className="checklist">
          {actions.map((a) => (
            <li key={a.title} className={a.done ? "done" : ""}>
              <div>
                <strong>{a.title}</strong>
                <p className="muted small">{a.sub}</p>
              </div>
              <button
                type="button"
                className="ghost"
                data-testid={`overview-cta-${a.cta.toLowerCase().replace(/\s+/g, "-")}`}
                onClick={a.onClick}
              >
                {a.cta}
              </button>
            </li>
          ))}
        </ul>
      </section>

      {recent.length > 0 && (
        <section className="card">
          <h3>Recent Campaigns</h3>
          <table>
            <thead>
              <tr>
                <th>Name</th>
                <th>Status</th>
                <th>Sent</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {recent.map((c) => (
                <tr key={c.id}>
                  <td>{c.name}</td>
                  <td>{c.status}</td>
                  <td>{c.sent}</td>
                  <td>
                    <button
                      type="button"
                      className="linkish"
                      onClick={() => onGoResults(c.id)}
                    >
                      Results
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}

      <section className="card lifecycle">
        <div className="section-head">
          <h3 className="section-head-title">
            End assessment
            <Hint hint="Export and selectively purge as needed, clean hosts, then Archive (keeps everything inactive) or Delete (erases this engagement from the app database). Shared kit phishlet YAML is never removed by Delete." />
          </h3>
        </div>

        <div className="lifecycle-step">
          <div className="lifecycle-step-head">
            <strong className="label-with-hint">
              1 · Export bundle
              <Hint hint="Portable JSON of scope, targets, campaigns, results, and sessions. Redacted bundles mask credentials, tokens, and recipient emails — safe to attach to a report." />
            </strong>
            <label className="check">
              <input
                type="checkbox"
                checked={redactExport}
                onChange={(e) => setRedactExport(e.target.checked)}
              />
              Redact secrets &amp; emails
            </label>
          </div>
          <button
            type="button"
            className="ghost"
            data-testid="overview-export"
            disabled={!!busy}
            onClick={exportBundle}
          >
            {busy === "export" ? "Exporting…" : "Download export bundle"}
          </button>
        </div>

        <div className="lifecycle-step">
          <div className="lifecycle-step-head">
            <strong className="label-with-hint">
              2 · Selective purge
              <Hint hint="Delete only the data types you choose. This cannot be undone — export first." />
            </strong>
          </div>
          <div className="row wrap">
            <label className="check">
              <input
                type="checkbox"
                checked={purgeSessions}
                onChange={(e) => setPurgeSessions(e.target.checked)}
              />
              Captured sessions
            </label>
            <label className="check">
              <input
                type="checkbox"
                checked={purgeAttempts}
                onChange={(e) => setPurgeAttempts(e.target.checked)}
              />
              Send attempts
            </label>
            <label className="check">
              <input
                type="checkbox"
                checked={purgePii}
                onChange={(e) => setPurgePii(e.target.checked)}
              />
              Recipient lists / PII
            </label>
          </div>
          <button
            type="button"
            className="ghost danger"
            data-testid="overview-purge"
            disabled={!!busy}
            onClick={runPurge}
          >
            {busy === "purge" ? "Purging…" : "Purge selected data"}
          </button>
        </div>

        <div className="lifecycle-step">
          <div className="lifecycle-step-head">
            <strong className="label-with-hint">
              3 · Hosts cleanup
              <Hint hint="Remove the /etc/hosts entries phishkit added for this assessment's dry-run domains (one admin prompt)." />
            </strong>
          </div>
          <button
            type="button"
            className="ghost"
            data-testid="overview-hosts-cleanup"
            disabled={!!busy}
            onClick={runHostsCleanup}
          >
            {busy === "hosts" ? "Cleaning…" : "Clean /etc/hosts entries"}
          </button>
        </div>

        <div className="lifecycle-step">
          <div className="lifecycle-step-head">
            <strong className="label-with-hint">
              {isArchived ? "4 · Restore / New from archive" : "4 · Archive"}
              <Hint
                hint={
                  isArchived
                    ? "Restore returns this engagement to the active list. New from archive creates a fresh active copy with Targets/Lures (not campaigns or sessions)."
                    : "Marks the engagement inactive — data stays in the database. Find it under Assessments → Show archived. Stops the proxy first if it is still live."
                }
              />
            </strong>
            {!isArchived && (
              <label className="check">
                <input
                  type="checkbox"
                  checked={cleanupHosts}
                  onChange={(e) => setCleanupHosts(e.target.checked)}
                />
                Also clean /etc/hosts
              </label>
            )}
          </div>
          {isArchived ? (
            <div className="row wrap">
              <button
                type="button"
                data-testid="overview-restore"
                disabled={!!busy}
                onClick={restoreAssessment}
              >
                {busy === "unarchive" ? "Restoring…" : "Restore assessment"}
              </button>
              <button
                type="button"
                className="ghost"
                data-testid="overview-clone"
                disabled={!!busy}
                onClick={cloneAssessment}
              >
                {busy === "clone" ? "Creating…" : "New from archive"}
              </button>
            </div>
          ) : (
            <button
              type="button"
              className="ghost"
              data-testid="overview-archive"
              disabled={!!busy}
              onClick={archiveAssessment}
            >
              {busy === "archive" ? "Archiving…" : "Archive assessment"}
            </button>
          )}
        </div>

        {isArchived && (
          <div className="lifecycle-step">
            <div className="lifecycle-step-head">
              <strong className="label-with-hint">
                5 · Delete permanently
                <Hint hint="Erases this assessment and all engagement-owned rows from the app database. Not undoable. Export first if you need evidence. Shared kit/evilginx/phishlets YAML is kept." />
              </strong>
              <label className="check">
                <input
                  type="checkbox"
                  checked={cleanupHosts}
                  onChange={(e) => setCleanupHosts(e.target.checked)}
                />
                Also clean /etc/hosts
              </label>
            </div>
            <button
              type="button"
              className="ghost danger"
              data-testid="overview-delete"
              disabled={!!busy}
              onClick={deleteAssessment}
            >
              {busy === "delete" ? "Deleting…" : "Delete assessment"}
            </button>
          </div>
        )}
      </section>
    </div>
  );
}
