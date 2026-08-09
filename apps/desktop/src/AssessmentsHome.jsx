import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { fmtAssessment } from "./lib/workspace";
import Hint from "./components/Hint";
import EmptyState from "./components/EmptyState";
import { IconGrid } from "./lib/icons";
import { useModalBehavior } from "./lib/modal";

const EMPTY_FORM = {
  name: "",
  primaryDomain: "",
  authorizedBy: "",
  authorizationRef: "",
  notes: "",
};

export default function AssessmentsHome({
  assessments,
  showArchived = false,
  onShowArchivedChange,
  onOpen,
  onCreated,
  append,
  busy,
  setBusy,
  refreshList,
  onStartDemoTour,
  tourOpenNewTick = 0,
  showDemoTourBanner = true,
}) {
  const [showNew, setShowNew] = useState(false);
  const [form, setForm] = useState(EMPTY_FORM);
  const [touched, setTouched] = useState({});
  const dialogRef = useRef(null);

  const closeNew = useCallback(() => {
    if (busy) return;
    setShowNew(false);
    setTouched({});
  }, [busy]);

  useModalBehavior(showNew, closeNew, dialogRef);

  useEffect(() => {
    if (tourOpenNewTick) setShowNew(true);
  }, [tourOpenNewTick]);

  const setField = (key) => (e) =>
    setForm((f) => ({ ...f, [key]: e.target.value }));
  const markTouched = (key) => () =>
    setTouched((t) => (t[key] ? t : { ...t, [key]: true }));

  const nameErr = form.name.trim() ? "" : "Name is required";
  const domainErr = form.primaryDomain.trim()
    ? ""
    : "Primary domain or URL is required";
  const valid = !nameErr && !domainErr;

  const submitNew = async () => {
    const name = form.name.trim();
    const primaryDomain = form.primaryDomain.trim();
    if (!name || !primaryDomain) {
      setTouched({ name: true, primaryDomain: true });
      return;
    }
    setBusy("create-assessment");
    try {
      const created = await invoke("cmd_create_assessment", {
        req: {
          name,
          primaryDomain,
          authorizedBy: form.authorizedBy.trim() || null,
          authorizationRef: form.authorizationRef.trim() || null,
          notes: form.notes.trim() || null,
        },
      });
      await invoke("cmd_set_active_assessment", { id: created.id });
      const normalized = fmtAssessment(created);
      setShowNew(false);
      setTouched({});
      setForm(EMPTY_FORM);
      append(`Assessment "${normalized.name}" created`);
      await refreshList?.();
      onCreated(normalized);
    } catch (e) {
      append(String(e));
    } finally {
      setBusy("");
    }
  };

  const openAssessment = async (a) => {
    setBusy("open-assessment");
    try {
      if ((a.status || "").toLowerCase() !== "archived") {
        await invoke("cmd_set_active_assessment", { id: a.id });
      }
      onOpen(fmtAssessment(a));
    } catch (e) {
      append(String(e));
    } finally {
      setBusy("");
    }
  };

  const archiveAssessment = async (a, e) => {
    e?.stopPropagation?.();
    e?.preventDefault?.();
    const ok = window.confirm(
      `Archive assessment "${a.name}"?\n\n` +
        `Archive does NOT delete data — it marks the engagement inactive. ` +
        `Find it under “Show archived” to Restore, New from archive, or Delete.`
    );
    if (!ok) return;
    setBusy("archive-assessment");
    try {
      try {
        await invoke("evilginx_stop");
      } catch {
        /* best effort — proxy may already be idle */
      }
      await invoke("cmd_archive_assessment", { id: a.id });
      append(`Archived assessment "${a.name}"`);
      await refreshList?.();
    } catch (err) {
      append(String(err));
    } finally {
      setBusy("");
    }
  };

  const restoreAssessment = async (a, e) => {
    e?.stopPropagation?.();
    e?.preventDefault?.();
    setBusy("unarchive-assessment");
    try {
      const restored = await invoke("cmd_unarchive_assessment", { id: a.id });
      append(`Restored assessment "${restored.name || a.name}"`);
      await refreshList?.();
    } catch (err) {
      append(String(err));
    } finally {
      setBusy("");
    }
  };

  const cloneAssessment = async (a, e) => {
    e?.stopPropagation?.();
    e?.preventDefault?.();
    setBusy("clone-assessment");
    try {
      const created = await invoke("cmd_clone_assessment", { id: a.id });
      await invoke("cmd_set_active_assessment", { id: created.id });
      append(
        `Created “${created.name}” from ${a.name} (Targets copied; campaigns/sessions not copied)`
      );
      await refreshList?.();
      onCreated?.(fmtAssessment(created));
    } catch (err) {
      append(String(err));
    } finally {
      setBusy("");
    }
  };

  const deleteAssessment = async (a, e) => {
    e?.stopPropagation?.();
    e?.preventDefault?.();
    const ok = window.confirm(
      `DELETE assessment "${a.name}" permanently?\n\n` +
        `This removes Targets, Lures, Campaigns, Sessions, templates, and recipient lists ` +
        `for this engagement from the app database. It cannot be undone.\n\n` +
        `Shared phishlet YAML under kit/evilginx/phishlets/ is not deleted.`
    );
    if (!ok) return;
    const ok2 = window.confirm(
      `Type-confirm: permanently delete “${a.name}”? There is no recycle bin.`
    );
    if (!ok2) return;
    setBusy("delete-assessment");
    try {
      try {
        await invoke("evilginx_stop");
      } catch {
        /* best effort */
      }
      try {
        await invoke("cmd_assessment_hosts_cleanup", { id: a.id });
      } catch {
        /* best effort */
      }
      const r = await invoke("cmd_delete_assessment", { id: a.id });
      append(
        `Deleted assessment “${r.name}” · ${r.profilesDeleted ?? r.profiles_deleted ?? 0} targets · ` +
          `${r.campaignsDeleted ?? r.campaigns_deleted ?? 0} campaigns`
      );
      await refreshList?.();
    } catch (err) {
      append(String(err));
    } finally {
      setBusy("");
    }
  };

  const activeCount = assessments.filter(
    (a) => (a.status || "active").toLowerCase() !== "archived"
  ).length;
  const archivedCount = assessments.filter(
    (a) => (a.status || "").toLowerCase() === "archived"
  ).length;

  return (
    <div className="assessments-home">
      <div className="assessments-head">
        <h2 className="section-head-title">
          Assessments
          <Hint hint="Archive keeps the engagement in the database as inactive (Show archived → Restore or New from archive). Delete permanently erases all assessment data from the app database — not undoable." />
        </h2>
        <div className="assessments-head-actions">
          <label className="check assessments-archived-toggle">
            <input
              type="checkbox"
              data-testid="assessments-show-archived"
              checked={!!showArchived}
              disabled={!!busy}
              onChange={(e) => onShowArchivedChange?.(e.target.checked)}
            />
            Show archived
            {showArchived && archivedCount ? ` (${archivedCount})` : ""}
          </label>
          <button
            type="button"
            className="ghost"
            data-testid="start-demo-tour-home"
            disabled={!!busy}
            onClick={() => onStartDemoTour?.()}
          >
            Demo tour
          </button>
          <button
            type="button"
            data-testid="new-assessment"
            disabled={!!busy}
            onClick={() => setShowNew(true)}
          >
            New assessment
          </button>
        </div>
      </div>

      {showDemoTourBanner && !assessments.length && !showNew && (
        <div className="demo-tour-banner" data-testid="demo-tour-banner">
          <p>
            New here? Take the interactive demo tour — Assessment → demo Target → Community
            phishlets → Campaigns → Sessions.
          </p>
          <button type="button" data-testid="demo-tour-banner-start" onClick={() => onStartDemoTour?.()}>
            Start demo tour
          </button>
        </div>
      )}

      {!assessments.length && !showNew && (
        <EmptyState
          icon={<IconGrid size={22} />}
          title={
            showArchived ? "No assessments (including archived)" : "No assessments yet"
          }
          action={
            <button
              type="button"
              data-testid="new-assessment-empty"
              disabled={!!busy}
              onClick={() => setShowNew(true)}
            >
              Create your first assessment
            </button>
          }
        >
          {showArchived
            ? "Archived engagements appear here when any exist."
            : "Scope Targets, configure AiTM proxy Lures, and run authorized Campaigns."}
        </EmptyState>
      )}

      {!activeCount && !!archivedCount && !showArchived && !showNew && (
        <p className="muted assessments-archived-hint">
          All assessments are archived.{" "}
          <button
            type="button"
            className="linkish"
            onClick={() => onShowArchivedChange?.(true)}
          >
            Show archived
          </button>{" "}
          to restore one.
        </p>
      )}

      <ul className="assessment-grid">
        {assessments.map((raw) => {
          const a = fmtAssessment(raw);
          const archived = (a.status || "").toLowerCase() === "archived";
          return (
            <li key={a.id} className={`assessment-card card${archived ? " is-archived" : ""}`}>
              <button
                type="button"
                className="assessment-card-main"
                data-testid={`assessment-card-${a.id}`}
                disabled={!!busy}
                onClick={() => openAssessment(a)}
              >
                <div className="assessment-card-head">
                  <h3>{a.name}</h3>
                  <span className={`tag status-${(a.status || "active").toLowerCase()}`}>
                    {a.status || "active"}
                  </span>
                </div>
                <p className="mono small">{a.primaryDomain || "—"}</p>
                <dl className="assessment-stats">
                  <div>
                    <dt>Targets</dt>
                    <dd>{a.targetCount}</dd>
                  </div>
                  <div>
                    <dt>Campaigns</dt>
                    <dd>{a.campaignCount}</dd>
                  </div>
                  <div>
                    <dt>Sessions</dt>
                    <dd>{a.sessionCount}</dd>
                  </div>
                </dl>
              </button>
              <div className="assessment-card-actions">
                {archived ? (
                  <>
                    <button
                      type="button"
                      className="ghost"
                      data-testid={`assessment-restore-${a.id}`}
                      disabled={!!busy}
                      onClick={(e) => restoreAssessment(a, e)}
                    >
                      Restore
                    </button>
                    <button
                      type="button"
                      className="ghost"
                      data-testid={`assessment-clone-${a.id}`}
                      disabled={!!busy}
                      onClick={(e) => cloneAssessment(a, e)}
                      title="New active assessment with copied Targets (not campaigns/sessions)"
                    >
                      New from archive
                    </button>
                    <button
                      type="button"
                      className="ghost danger"
                      data-testid={`assessment-delete-${a.id}`}
                      disabled={!!busy}
                      onClick={(e) => deleteAssessment(a, e)}
                    >
                      Delete
                    </button>
                  </>
                ) : (
                  <button
                    type="button"
                    className="ghost"
                    data-testid={`assessment-archive-${a.id}`}
                    disabled={!!busy}
                    onClick={(e) => archiveAssessment(a, e)}
                  >
                    Archive
                  </button>
                )}
              </div>
            </li>
          );
        })}
      </ul>

      {showNew && (
        <>
          <div className="modal-overlay" onClick={closeNew} />
          <div
            className="modal card"
            role="dialog"
            aria-modal="true"
            aria-label="New assessment"
            ref={dialogRef}
          >
            <h3>New assessment</h3>
            <label className="block">
              Name
              <input
                data-testid="assessment-name"
                value={form.name}
                onChange={setField("name")}
                onBlur={markTouched("name")}
                placeholder="Q3 authorized assessment"
                aria-invalid={touched.name && !!nameErr}
                autoFocus
              />
              {touched.name && nameErr ? (
                <span className="field-err">{nameErr}</span>
              ) : null}
            </label>
            <label className="block">
              <span className="label-with-hint">
                Primary domain or URL
                <Hint hint="The domain you're authorized to assess. Targets and dry-run domains derive from this." />
              </span>
              <input
                data-testid="assessment-domain"
                className="mono"
                value={form.primaryDomain}
                onChange={setField("primaryDomain")}
                onBlur={markTouched("primaryDomain")}
                placeholder="app.client.com"
                aria-invalid={touched.primaryDomain && !!domainErr}
              />
              {touched.primaryDomain && domainErr ? (
                <span className="field-err">{domainErr}</span>
              ) : null}
            </label>
            <details className="compliance-details">
              <summary>Compliance details (optional)</summary>
              <div className="fields">
                <label className="block">
                  Authorized by
                  <input
                    data-testid="assessment-authorized-by"
                    value={form.authorizedBy}
                    onChange={setField("authorizedBy")}
                    placeholder="Security lead"
                  />
                </label>
                <label className="block">
                  Authorization reference
                  <input
                    data-testid="assessment-auth-ref"
                    value={form.authorizationRef}
                    onChange={setField("authorizationRef")}
                    placeholder="Ticket / SOW #"
                  />
                </label>
              </div>
              <label className="block">
                Notes
                <textarea
                  data-testid="assessment-notes"
                  rows={3}
                  value={form.notes}
                  onChange={setField("notes")}
                  placeholder="Scope notes, constraints…"
                />
              </label>
            </details>
            <div className="row">
              <button
                type="button"
                data-testid="assessment-create"
                disabled={!!busy || !valid}
                onClick={submitNew}
              >
                {busy === "create-assessment" ? "Creating…" : "Create & open"}
              </button>
              <button
                type="button"
                className="ghost"
                disabled={!!busy}
                onClick={closeNew}
              >
                Cancel
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
