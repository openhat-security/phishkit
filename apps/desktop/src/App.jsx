import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import AssessmentsHome from "./AssessmentsHome";
import SetupWizard from "./SetupWizard";
import SettingsView from "./SettingsView";
import AssessmentOverview from "./AssessmentOverview";
import Destinations from "./Destinations";
import Sessions from "./Sessions";
import Hint from "./components/Hint";
import EmptyState from "./components/EmptyState";
import ErrorBoundary from "./components/ErrorBoundary";
import DemoTour, { shouldAutoOfferDemoTour } from "./components/DemoTour";
import { useModalBehavior } from "./lib/modal";
import {
  DEMO_TOUR_STATUS,
  saveDemoTourState,
} from "./lib/demoTour";
import {
  CampaignsView,
  DeliverySettingsView,
  RecipientsView,
  ResultsView,
  TemplatesView,
} from "./MailViews";
import {
  ASSESSMENT_NAV,
  HOME_NAV,
  TARGET_NAV,
  assessmentMeta,
  fmtAssessment,
  fmtProfile,
  homeMeta,
  targetMeta,
} from "./lib/workspace";
import {
  IconChart,
  IconHook,
  IconMail,
  IconSend,
  IconSliders,
  IconTarget,
  IconTerminal,
  IconUsers,
  IconX,
} from "./lib/icons";
import "./App.css";

const ASSESSMENT_ICONS = {
  overview: IconTarget,
  targets: IconTarget,
  templates: IconMail,
  recipients: IconUsers,
  campaigns: IconSend,
  results: IconChart,
  delivery: IconSliders,
};

export default function App() {
  const [mode, setMode] = useState("home");
  const [activeAssessment, setActiveAssessment] = useState(null);
  const [activeTarget, setActiveTarget] = useState(null);
  const [assessments, setAssessments] = useState([]);
  const [showArchivedAssessments, setShowArchivedAssessments] = useState(false);
  const [targets, setTargets] = useState([]);
  const [nav, setNav] = useState("assessments");

  const [busy, setBusy] = useState("");
  const [log, setLog] = useState("");
  const [kit, setKit] = useState(null);
  const [status, setStatus] = useState(null);
  const [campaignPrefill, setCampaignPrefill] = useState(null);
  const [resultsCampaignId, setResultsCampaignId] = useState("");
  const [sessionFocusId, setSessionFocusId] = useState("");
  const [runtimeProfileId, setRuntimeProfileId] = useState(null);

  const [activityOpen, setActivityOpen] = useState(false);
  const [unread, setUnread] = useState(0);
  const activityOpenRef = useRef(false);

  const [newTargetUrl, setNewTargetUrl] = useState("");
  const [newTargetName, setNewTargetName] = useState("");
  const [creatingTarget, setCreatingTarget] = useState(false);

  const [demoTourOpen, setDemoTourOpen] = useState(false);
  const [demoTourStep, setDemoTourStep] = useState(0);
  const [reconView, setReconView] = useState("proxy");
  const [tourOpenNewTick, setTourOpenNewTick] = useState(0);
  const [sessionReady, setSessionReady] = useState(false);
  const [setup, setSetup] = useState(null);
  const [persona, setPersona] = useState("cybersecStudent");
  const demoTourOfferedRef = useRef(false);

  const append = useCallback((msg) => {
    setLog((p) =>
      `${new Date().toLocaleTimeString()}  ${msg}\n${p}`.slice(0, 12000)
    );
    if (!activityOpenRef.current) setUnread((u) => u + 1);
  }, []);


  useEffect(() => {
    (async () => {
      try {
        const s = await invoke("cmd_get_setup");
        setSetup(s);
        setPersona(s.persona || "cybersecStudent");
      } catch (e) {
        append(String(e));
        setSetup({ setupComplete: false });
      }
    })();
  }, [append]);


  const activityRef = useRef(null);
  const openActivity = () => {
    activityOpenRef.current = true;
    setActivityOpen(true);
    setUnread(0);
  };
  const closeActivity = useCallback(() => {
    activityOpenRef.current = false;
    setActivityOpen(false);
  }, []);

  useModalBehavior(activityOpen, closeActivity, activityRef);

  const refreshChrome = useCallback(async () => {
    try {
      const [k, s] = await Promise.all([
        invoke("get_kit_info"),
        invoke("get_service_status"),
      ]);
      setKit(k);
      setStatus(s);
      try {
        setRuntimeProfileId(await invoke("get_runtime_profile"));
      } catch {
        setRuntimeProfileId(null);
      }
    } catch (e) {
      append(String(e));
    }
  }, [append]);

  const refreshAssessments = useCallback(async () => {
    try {
      const list = await invoke("cmd_list_assessments", {
        includeArchived: showArchivedAssessments,
      });
      setAssessments(list);
    } catch (e) {
      append(String(e));
    }
  }, [append, showArchivedAssessments]);

  const loadTargets = useCallback(async (assessmentId) => {
    if (!assessmentId) {
      setTargets([]);
      return;
    }
    try {
      const rows = await invoke("cmd_list_targets", { assessmentId });
      setTargets(rows.map(fmtProfile));
    } catch (e) {
      append(String(e));
    }
  }, [append]);

  const restoreSession = useCallback(async () => {
    try {
      await refreshAssessments();
      const active = await invoke("cmd_get_active_assessment");
      if (active) {
        const assessment = fmtAssessment(active);
        if ((assessment.status || "").toLowerCase() !== "archived") {
          setActiveAssessment(assessment);
          setMode("assessment");
          setNav("overview");
          await loadTargets(assessment.id);

          const profileId = await invoke("get_active_profile");
          if (profileId) {
            const p = await invoke("get_profile", { id: profileId });
            if (p) {
              const target = fmtProfile(p);
              if (!target.assessmentId || target.assessmentId === assessment.id) {
                setActiveTarget(target);
                setMode("target");
                setNav("overview");
              }
            }
          }
        }
      }
    } catch (e) {
      append(String(e));
    } finally {
      setSessionReady(true);
    }
  }, [append, loadTargets, refreshAssessments]);

  useEffect(() => {
    restoreSession();
    refreshChrome();
    const t = setInterval(refreshChrome, 6000);
    return () => clearInterval(t);
  }, [restoreSession, refreshChrome]);

  useEffect(() => {
    if (mode === "assessment" && activeAssessment?.id) {
      loadTargets(activeAssessment.id);
    }
  }, [mode, activeAssessment?.id, loadTargets]);

  useEffect(() => {
    if (mode === "home" && nav === "assessments") {
      refreshAssessments();
    }
  }, [showArchivedAssessments, mode, nav, refreshAssessments]);

  const proxyUp = !!status?.evilginx_running;
  const binReady = !!kit?.evilginx_bin;
  const runtimeTarget =
    (runtimeProfileId &&
      (targets.find((t) => t.id === runtimeProfileId) ||
        (activeTarget?.id === runtimeProfileId ? activeTarget : null))) ||
    null;
  const runtimeLabel =
    runtimeTarget?.name ||
    runtimeTarget?.targetDomain ||
    (runtimeProfileId ? String(runtimeProfileId).slice(0, 8) : null);

  const goHome = useCallback(() => {
    setMode("home");
    setNav("assessments");
    setActiveTarget(null);
  }, []);

  const startDemoTour = useCallback(() => {
    setDemoTourStep(0);
    setDemoTourOpen(true);
    saveDemoTourState({ status: DEMO_TOUR_STATUS.active, step: 0 });
  }, []);

  const closeDemoTour = useCallback((status) => {
    setDemoTourOpen(false);
    if (status) {
      saveDemoTourState({ status, step: demoTourStep });
      invoke("cmd_set_tutorial_completed", { done: true }).then((s) => {
        setSetup(s);
      }).catch(() => {});
    }
  }, [demoTourStep]);

  useEffect(() => {
    if (!sessionReady || !setup?.setupComplete || demoTourOfferedRef.current) return;
    if (setup.tutorialCompleted) {
      demoTourOfferedRef.current = true;
      return;
    }
    demoTourOfferedRef.current = true;
    if (shouldAutoOfferDemoTour(assessments.length)) {
      startDemoTour();
    }
  }, [sessionReady, setup, assessments.length, startDemoTour]);

  const enterAssessment = (assessment) => {
    setActiveAssessment(fmtAssessment(assessment));
    setActiveTarget(null);
    setMode("assessment");
    setNav("overview");
  };

  const openTarget = async (target) => {
    const t = fmtProfile(target);
    setBusy("open-target");
    try {
      await invoke("activate_profile", { id: t.id });
      setActiveTarget(t);
      setMode("target");
      setNav("recon");
    } catch (e) {
      append(String(e));
    } finally {
      setBusy("");
    }
  };

  const backToAssessment = () => {
    setActiveTarget(null);
    setMode("assessment");
    setNav("overview");
  };

  // A Target is opened from the assessment's Targets list, so "back" should
  // return there (not the Overview).
  const backToTargets = () => {
    setActiveTarget(null);
    setMode("assessment");
    setNav("targets");
  };

  const createTarget = async () => {
    const url = newTargetUrl.trim();
    if (!url || !activeAssessment?.id) {
      append("Enter a target URL or domain");
      return;
    }
    setBusy("create-target");
    try {
      const r = await invoke("ensure_destination", {
        target: url,
        name: newTargetName.trim() || null,
        overwrite: false,
        assessmentId: activeAssessment.id,
      });
      await invoke("activate_profile", { id: r.profile.id });
      append(r.message || "Target ready");
      setNewTargetUrl("");
      setNewTargetName("");
      setCreatingTarget(false);
      await loadTargets(activeAssessment.id);
      await refreshAssessments();
      openTarget(r.profile);
    } catch (e) {
      append(String(e));
    } finally {
      setBusy("");
    }
  };

  const goCampaignFromDestination = ({ linkUrl, profileId }) => {
    setCampaignPrefill({ linkUrl, profileId, ts: Date.now() });
    if (mode === "target") {
      setMode("assessment");
      setNav("campaigns");
    } else {
      setNav("campaigns");
    }
  };

  const goResults = (campaignId) => {
    if (campaignId) setResultsCampaignId(campaignId);
    if (mode === "target") {
      setMode("assessment");
      setNav("results");
    } else {
      setNav("results");
    }
  };

  const leaveToHome = async () => {
    setActiveTarget(null);
    setActiveAssessment(null);
    setMode("home");
    setNav("assessments");
    await refreshAssessments();
  };

  const openSessionFromResults = async ({ sessionId, profileId }) => {
    try {
      let target =
        (profileId && targets.find((t) => t.id === profileId)) ||
        (profileId ? fmtProfile(await invoke("get_profile", { id: profileId })) : null) ||
        activeTarget;
      if (!target) {
        append(`Session ${sessionId} — open a Target to inspect captures`);
        return;
      }
      if (profileId) {
        await invoke("activate_profile", { id: profileId }).catch(() => {});
      }
      setActiveTarget(target);
      setSessionFocusId(sessionId ? String(sessionId) : "");
      setMode("target");
      setNav("sessions");
      append(`Opened Sessions for ${target.name || target.targetDomain} · ${sessionId}`);
    } catch (e) {
      append(String(e));
    }
  };

  const pageMeta = (() => {
    if (mode === "home") return homeMeta(nav);
    if (mode === "assessment") return assessmentMeta(nav);
    return targetMeta(nav);
  })();

  const renderSidebar = () => {
    if (mode === "home") {
      return (
        <>
          <div className="nav-section">Home</div>
          {HOME_NAV.map(({ id, label }) => (
            <button
              key={id}
              type="button"
              data-testid={`nav-${id}`}
              className={`nav-item ${nav === id ? "active" : ""}`}
              onClick={() => setNav(id)}
            >
              {label}
            </button>
          ))}
        </>
      );
    }

    if (mode === "assessment") {
      return (
        <>
          <div className="nav-section">Assessment</div>
          {ASSESSMENT_NAV.map(({ id, label }) => {
            const Icon = ASSESSMENT_ICONS[id] || IconTarget;
            return (
              <button
                key={id}
                type="button"
                data-testid={`nav-${id}`}
                className={`nav-item ${nav === id ? "active" : ""}`}
                onClick={() => setNav(id)}
              >
                <span className="ico">
                  <Icon size={17} />
                </span>
                {label}
              </button>
            );
          })}
        </>
      );
    }

    return (
      <>
        <div className="nav-section">Target</div>
        {TARGET_NAV.map(({ id, label }) => (
          <button
            key={id}
            type="button"
            data-testid={`nav-${id}`}
            className={`nav-item ${nav === id ? "active" : ""}`}
            onClick={() => setNav(id)}
          >
            {label}
          </button>
        ))}
      </>
    );
  };

  // Persistent workspace context at the top of the sidebar: the active
  // assessment is always shown (with a Switch control); inside a Target a
  // second row appears with a Back-to-targets control. Replaces the buttons
  // that used to sit at the bottom of the sidebar.
  const renderContext = () => {
    if (mode === "home" || !activeAssessment) return null;
    return (
      <div className="ctx">
        <div className="ctx-row">
          <button
            type="button"
            className="ctx-main"
            data-testid="ctx-assessment"
            onClick={backToAssessment}
            title="Assessment overview"
          >
            <span className="ctx-eyebrow">Assessment</span>
            <span className="ctx-name truncate" title={activeAssessment.name}>
              {activeAssessment.name}
            </span>
            {activeAssessment.primaryDomain ? (
              <span
                className="ctx-sub mono truncate"
                title={activeAssessment.primaryDomain}
              >
                {activeAssessment.primaryDomain}
              </span>
            ) : null}
          </button>
          <button
            type="button"
            className="ctx-switch"
            data-testid="ctx-switch-assessment"
            onClick={goHome}
            title="Switch assessment"
            aria-label="Switch assessment"
          >
            ⇄
          </button>
        </div>
        {mode === "target" && activeTarget && (
          <div className="ctx-row ctx-nested">
            <button
              type="button"
              className="ctx-main"
              data-testid="ctx-target"
              onClick={() => setNav("overview")}
              title="Target overview"
            >
              <span className="ctx-eyebrow">Target</span>
              <span
                className="ctx-name truncate"
                title={activeTarget.name || activeTarget.targetDomain}
              >
                {activeTarget.name || activeTarget.targetDomain}
              </span>
            </button>
            <button
              type="button"
              className="ctx-switch"
              data-testid="ctx-back-targets"
              onClick={backToTargets}
              title="Back to targets"
              aria-label="Back to targets"
            >
              ←
            </button>
          </div>
        )}
      </div>
    );
  };

  const renderBreadcrumb = () => {
    if (mode === "home") return null;
    if (mode === "assessment" && activeAssessment) {
      return (
        <nav className="breadcrumb" aria-label="Assessment path">
          <button
            type="button"
            className="linkish"
            data-testid="breadcrumb-assessments"
            onClick={goHome}
          >
            Assessments
          </button>
          <span className="sep">/</span>
          <span>
            {activeAssessment.name}{" "}
            <span className="muted">({activeAssessment.primaryDomain})</span>
          </span>
        </nav>
      );
    }
    if (mode === "target" && activeAssessment && activeTarget) {
      return (
        <nav className="breadcrumb" aria-label="Target path">
          <button
            type="button"
            className="linkish"
            data-testid="breadcrumb-assessments"
            onClick={goHome}
          >
            Assessments
          </button>
          <span className="sep">/</span>
          <button type="button" className="linkish" onClick={backToAssessment}>
            {activeAssessment.name}
          </button>
          <span className="sep">/</span>
          <button type="button" className="linkish" onClick={backToTargets}>
            Targets
          </button>
          <span className="sep">/</span>
          <span>{activeTarget.name || activeTarget.targetDomain}</span>
        </nav>
      );
    }
    return null;
  };

  const renderContent = () => {
    if (mode === "home") {
      if (nav === "delivery") {
        return (
          <DeliverySettingsView busy={busy} setBusy={setBusy} append={append} />
        );
      }
      if (nav === "settings") {
        return (
          <SettingsView
            append={append}
            onStartTutorial={startDemoTour}
            onSetupChanged={async () => {
              try {
                const s = await invoke("cmd_get_setup");
                setSetup(s);
                setPersona(s.persona || "cybersecStudent");
              } catch (e) {
                append(String(e));
              }
            }}
          />
        );
      }
      return (
        <AssessmentsHome
          assessments={assessments}
          showArchived={showArchivedAssessments}
          onShowArchivedChange={setShowArchivedAssessments}
          busy={busy}
          setBusy={setBusy}
          append={append}
          refreshList={refreshAssessments}
          onOpen={enterAssessment}
          onCreated={enterAssessment}
          onStartDemoTour={startDemoTour}
          tourOpenNewTick={tourOpenNewTick}
          showDemoTourBanner={!demoTourOpen}
        />
      );
    }

    if (mode === "assessment") {
      const aid = activeAssessment?.id;
      switch (nav) {
        case "overview":
          return (
            <AssessmentOverview
              assessment={activeAssessment}
              status={status}
              append={append}
              busy={busy}
              setBusy={setBusy}
              onGoTargets={() => setNav("targets")}
              onGoCampaigns={() => setNav("campaigns")}
              onGoResults={goResults}
              onOpenTarget={openTarget}
              onArchived={leaveToHome}
              onRestored={(restored) => {
                setActiveAssessment(fmtAssessment(restored));
                refreshAssessments();
              }}
              onDeleted={leaveToHome}
              onCloned={(created) => {
                enterAssessment(fmtAssessment(created));
                refreshAssessments();
              }}
            />
          );
        case "targets":
          return (
            <section className="card">
              <div className="sites-header">
                <h2 className="section-head-title">
                  Targets
                  <Hint hint="Each Target is a site profile with its own Phishlet, Lure, and captured Sessions." />
                </h2>
                <button
                  type="button"
                  data-testid="new-target"
                  disabled={!!busy}
                  onClick={() => setCreatingTarget((v) => !v)}
                >
                  {creatingTarget ? "Cancel" : "New target"}
                </button>
              </div>
              {creatingTarget && (
                <div className="site-create">
                  <label className="block">
                    Target URL or domain
                    <input
                      data-testid="target-url"
                      value={newTargetUrl}
                      onChange={(e) => setNewTargetUrl(e.target.value)}
                      placeholder="https://app.client.com"
                      autoFocus
                    />
                  </label>
                  <label className="block">
                    Name <span className="muted small">(optional)</span>
                    <input
                      data-testid="target-name"
                      value={newTargetName}
                      onChange={(e) => setNewTargetName(e.target.value)}
                    />
                  </label>
                  <button
                    type="button"
                    data-testid="target-create"
                    disabled={!!busy || !newTargetUrl.trim()}
                    onClick={createTarget}
                  >
                    {busy === "create-target" ? "Detecting…" : "Detect & create Target"}
                  </button>
                </div>
              )}
              <ul className="site-list" data-testid="target-list">
                {targets.map((t) => (
                  <li key={t.id}>
                    <button
                      type="button"
                      data-testid={`target-row-${t.id}`}
                      className="site-row"
                      onClick={() => openTarget(t)}
                    >
                      <span className="site-name">{t.name}</span>
                      <span className="mono small">{t.targetDomain || "—"}</span>
                      <span className="mono small muted">{t.phishlet || "no phishlet"}</span>
                    </button>
                  </li>
                ))}
              </ul>
              {!targets.length && !creatingTarget && (
                <EmptyState
                  icon={<IconTarget size={22} />}
                  title="No targets yet"
                  action={
                    <button
                      type="button"
                      disabled={!!busy}
                      onClick={() => setCreatingTarget(true)}
                    >
                      Add your first target
                    </button>
                  }
                >
                  Add a URL — we detect the stack and generate a phishlet to start recon.
                </EmptyState>
              )}
            </section>
          );
        case "templates":
          return (
            <TemplatesView
              busy={busy}
              setBusy={setBusy}
              append={append}
              assessmentId={aid}
            />
          );
        case "recipients":
          return (
            <RecipientsView
              busy={busy}
              setBusy={setBusy}
              append={append}
              assessmentId={aid}
            />
          );
        case "campaigns":
          return (
            <CampaignsView
              showAdvancedFlows={persona === "developer" || persona === "penetrationTester"}
              busy={busy}
              setBusy={setBusy}
              append={append}
              prefill={campaignPrefill}
              onOpenResults={goResults}
              assessmentId={aid}
              activeTargetId={activeTarget?.id}
            />
          );
        case "results":
          return (
            <ResultsView
              append={append}
              initialCampaignId={resultsCampaignId}
              assessmentId={aid}
              onOpenSession={openSessionFromResults}
            />
          );
        case "delivery":
          return (
            <DeliverySettingsView busy={busy} setBusy={setBusy} append={append} />
          );
        default:
          return null;
      }
    }

    if (mode === "target" && activeTarget) {
      switch (nav) {
        case "overview":
          return (
            <section className="card">
              <h2>{activeTarget.name || activeTarget.targetDomain}</h2>
              <dl className="grid">
                <div>
                  <dt>Domain</dt>
                  <dd className="mono">{activeTarget.targetDomain || "—"}</dd>
                </div>
                <div>
                  <dt>Phishlet</dt>
                  <dd className="mono">{activeTarget.phishlet || "—"}</dd>
                </div>
                <div>
                  <dt>Lure</dt>
                  <dd className="mono small">{activeTarget.lureUrl || "—"}</dd>
                </div>
                <div>
                  <dt>AiTM proxy</dt>
                  <dd>{proxyUp ? "Live" : "Idle"}</dd>
                </div>
              </dl>
              <div className="row">
                <button
                  type="button"
                  data-testid="target-overview-recon"
                  onClick={() => setNav("recon")}
                >
                  Recon & Proxy
                </button>
                <button
                  type="button"
                  className="ghost"
                  data-testid="target-overview-sessions"
                  onClick={() => setNav("sessions")}
                >
                  Sessions
                </button>
              </div>
            </section>
          );
        case "recon":
          return (
            <Destinations
              busy={busy}
              setBusy={setBusy}
              append={append}
              kit={kit}
              status={status}
              refreshChrome={refreshChrome}
              onUseInCampaign={goCampaignFromDestination}
              onOpenResults={goResults}
              initialProfileId={activeTarget.id}
              hideSitesList
              assessmentId={activeAssessment?.id}
              initialView={reconView || "proxy"}
              forcedView={demoTourOpen ? reconView : ""}
              onViewChange={setReconView}
            />
          );
        case "sessions":
          return (
            <Sessions
              busy={busy}
              setBusy={setBusy}
              append={append}
              profileId={activeTarget.id}
              focusSessionId={sessionFocusId}
              onOpenResults={goResults}
            />
          );
        default:
          return null;
      }
    }

    return null;
  };

    if (setup && !setup.setupComplete) {
    return (
      <SetupWizard
        append={append}
        onComplete={(saved, opts) => {
          setSetup(saved);
          setPersona(saved.persona || "cybersecStudent");
          if (opts?.startTutorial) startDemoTour();
        }}
      />
    );
  }

  if (!setup) {
    return <div className="setup-wizard"><p className="muted">Loading…</p></div>;
  }

return (
    <div className="shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="logo">
            <IconHook size={17} />
          </span>
          <span className="name">
            <b>phish</b>kit
            <small>alpha · AiTM console</small>
          </span>
        </div>
        {renderContext()}
        {renderSidebar()}
        <div className="spacer" />
        <div className="sidebar-foot">
          <button
            type="button"
            className="ghost"
            data-testid="start-demo-tour"
            disabled={demoTourOpen}
            onClick={startDemoTour}
          >
            Demo tour
          </button>
          <div>Alpha — not production</div>
          <div>Authorized assessments only</div>
          <div className="mono">{kit?.root || "locating kit…"}</div>
        </div>
      </aside>

      <div className="main">
        <header className="topbar">
          <div>
            {renderBreadcrumb()}
            <h1 className="page-title">{pageMeta.label}</h1>
            {mode === "home" && nav === "assessments" && (
              <p className="page-sub">Pick or create an authorized Assessment</p>
            )}
            {mode === "assessment" && nav === "overview" && (
              <p className="page-sub">Engagement status at a glance</p>
            )}
            {mode === "target" && nav === "recon" && (
              <p className="page-sub">AiTM proxy, Phishlet & Lure for this Target</p>
            )}
            {mode === "target" && nav === "sessions" && (
              <p className="page-sub">Captured Sessions for this Target</p>
            )}
          </div>
          <div className="grow" />

          <span
            className="status-chip"
            title={
              proxyUp && runtimeLabel
                ? `AiTM proxy runtime Target: ${runtimeLabel}`
                : "AiTM reverse proxy"
            }
          >
            <span className={`dot ${proxyUp ? "up" : "down"}`} />
            {proxyUp ? (
              <>
                <b>Proxy</b> live
                {runtimeLabel ? (
                  <span className="chip-extra mono"> · {runtimeLabel}</span>
                ) : null}
              </>
            ) : (
              <>
                Proxy <b>idle</b>
              </>
            )}
          </span>
          <span
            className="status-chip"
            title={binReady ? "evilginx binary present" : "run make build-evilginx"}
          >
            <span className={`dot ${binReady ? "up" : "warn"}`} />
            {binReady ? "binary ready" : "build needed"}
          </span>

          <button
            type="button"
            className="icon-btn"
            data-testid="activity-log"
            title="Activity log"
            onClick={openActivity}
          >
            <IconTerminal size={17} />
            {unread > 0 && <span className="badge-dot" />}
          </button>
        </header>

        <main className="content" data-testid="main-content">
          <ErrorBoundary resetKey={`${mode}:${nav}`} onError={(e) => append(String(e))}>
            {renderContent()}
          </ErrorBoundary>
        </main>
      </div>

      <DemoTour
        open={demoTourOpen}
        stepIndex={demoTourStep}
        onStepChange={setDemoTourStep}
        onClose={closeDemoTour}
        ctx={{
          setMode,
          setNav,
          setReconView,
          goHome,
          openNewAssessment: () => setTourOpenNewTick((n) => n + 1),
          mode,
          nav,
          activeAssessment,
          activeTarget,
        }}
      />

      {activityOpen && (
        <>
          <div className="activity-overlay" onClick={closeActivity} />
          <div
            className="activity"
            data-testid="activity-dialog"
            role="dialog"
            aria-modal="true"
            aria-label="Activity log"
            ref={activityRef}
          >
            <div className="activity-head">
              <IconTerminal size={17} />
              <h3>Activity</h3>
              <span className="grow" />
              <button
                type="button"
                className="ghost"
                data-testid="activity-clear"
                onClick={() => setLog("")}
                disabled={!log}
              >
                Clear
              </button>
              <button
                type="button"
                className="icon-btn"
                data-testid="activity-close"
                onClick={closeActivity}
              >
                <IconX size={16} />
              </button>
            </div>
            <div className="activity-body">
              {log ? (
                log
              ) : (
                <span className="activity-empty">
                  No activity yet. Actions and errors will stream here.
                </span>
              )}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
