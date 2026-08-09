import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import AupGate from "./components/AupGate";
import { SCENARIO_PRESETS, presetById } from "./lib/presets";
import { clearDraft, loadDraft, saveDraft } from "./lib/draftState";

const STEPS = ["Scenario", "Target & lure", "Message", "Recipients", "Sender", "Review"];

function previewHtml(html) {
  return String(html || "")
    .replaceAll("{{first_name}}", "Alex")
    .replaceAll("{{email}}", "alex@example.com")
    .replaceAll("{{link}}", "https://example.test/lure/preview");
}

function countEmails(text) {
  return String(text || "")
    .split(/[\n,;]+/)
    .map((s) => s.trim())
    .filter((s) => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(s.replace(/^.*</, "").replace(/>.*$/, "")))
    .length;
}

function templateHtml(t) {
  return t?.htmlBody ?? t?.html_body ?? "";
}

function listCountOf(l) {
  return l?.recipientCount ?? l?.recipient_count ?? 0;
}

/// Layered UX: a step-by-step wizard with inferred safe defaults and inline
/// "why" guidance, sitting on top of the same native engine the Composer uses.
/// Advanced fields stay editable so expert operators keep full control.
///
/// In-progress state is persisted per assessment so navigating to another tab
/// (Recipients, Templates, …) and back does not discard the draft.
export default function GuidedCampaign({
  assessmentId = null,
  activeTargetId = null,
  append,
  busy,
  setBusy,
  onOpenResults,
  onRefresh,
}) {
  const storageKey = `guided.${assessmentId || "global"}`;
  const savedRef = useRef(null);
  if (savedRef.current === null) savedRef.current = loadDraft(storageKey);
  const saved = savedRef.current;
  // Once a campaign launches we clear storage and must not let the final
  // render re-persist the just-launched draft.
  const skipPersistRef = useRef(false);

  const [step, setStep] = useState(() => saved.step ?? 0);
  const [presetId, setPresetId] = useState(() => saved.presetId ?? "");
  const [profiles, setProfiles] = useState([]);
  const [accounts, setAccounts] = useState([]);
  const [templates, setTemplates] = useState([]);
  const [lists, setLists] = useState([]);
  const [profileId, setProfileId] = useState(() => saved.profileId ?? activeTargetId ?? "");
  const [targetLures, setTargetLures] = useState([]);
  const [lureId, setLureId] = useState(() => saved.lureId ?? "");
  const [linkUrl, setLinkUrl] = useState(() => saved.linkUrl ?? "");
  const [name, setName] = useState(() => saved.name ?? "");
  const [rate, setRate] = useState(() => saved.rate ?? 10);
  const [mode, setMode] = useState(() => saved.mode ?? "aitm");
  const [templateId, setTemplateId] = useState(() => saved.templateId ?? "");
  const [templateName, setTemplateName] = useState(() => saved.templateName ?? "");
  const [subject, setSubject] = useState(() => saved.subject ?? "");
  const [html, setHtml] = useState(() => saved.html ?? "");
  const [listId, setListId] = useState(() => saved.listId ?? "");
  const [emails, setEmails] = useState(() => saved.emails ?? "");
  const [activeAccountId, setActiveAccountId] = useState(() => saved.activeAccountId ?? "");
  const [aupOk, setAupOk] = useState(false);
  const [draftId, setDraftId] = useState(() => saved.draftId ?? "");
  const [review, setReview] = useState(null);
  const [advanced, setAdvanced] = useState(() => saved.advanced ?? false);

  const preset = useMemo(() => presetById(presetId), [presetId]);

  // Persist the in-progress draft on every change (single JSON blob per key).
  useEffect(() => {
    if (skipPersistRef.current) return;
    saveDraft(storageKey, {
      step,
      presetId,
      profileId,
      lureId,
      linkUrl,
      name,
      rate,
      mode,
      templateId,
      templateName,
      subject,
      html,
      listId,
      emails,
      activeAccountId,
      advanced,
      draftId,
    });
  }, [
    storageKey,
    step,
    presetId,
    profileId,
    lureId,
    linkUrl,
    name,
    rate,
    mode,
    templateId,
    templateName,
    subject,
    html,
    listId,
    emails,
    activeAccountId,
    advanced,
    draftId,
  ]);

  const loadLibrary = async () => {
    const profilesPromise = assessmentId
      ? invoke("cmd_list_targets", { assessmentId })
      : invoke("list_profiles");
    const [p, acc, tpl, ls] = await Promise.all([
      profilesPromise,
      invoke("list_mail_accounts").catch(() => []),
      invoke("list_email_templates", { assessmentId: assessmentId || null }).catch(() => []),
      invoke("list_recipient_lists", { assessmentId: assessmentId || null }).catch(() => []),
    ]);
    setProfiles(p);
    setAccounts(acc);
    setTemplates(tpl);
    setLists(ls);
    const active = acc.find((a) => a.active);
    // Keep a restored sender; only fall back to the active/first account.
    setActiveAccountId((cur) => cur || active?.id || acc[0]?.id || "");
    if (!profileId) {
      const pick =
        (activeTargetId && p.find((x) => x.id === activeTargetId)) ||
        p.find((x) => x.lure_url || x.lureUrl) ||
        p[0];
      if (pick) setProfileId(pick.id);
    }
  };

  useEffect(() => {
    loadLibrary().catch((e) => append(String(e)));
    invoke("get_aup_status")
      .then((s) => setAupOk(!!s.accepted))
      .catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [assessmentId, activeTargetId]);

  // A restored draft keeps its campaign id but not the (server-side) review;
  // re-fetch it so Launch is enabled without rebuilding.
  useEffect(() => {
    if (draftId && !review) {
      invoke("campaign_review", { campaignId: draftId })
        .then(setReview)
        .catch(() => {});
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [draftId]);

  useEffect(() => {
    if (!profileId) {
      setTargetLures([]);
      return;
    }
    invoke("cmd_list_lures", { profileId })
      .then((rows) => {
        setTargetLures(rows);
        const def = rows.find((r) => r.isDefault || r.is_default) || rows[0];
        if (def) {
          // Don't clobber a restored/edited lure or link.
          setLureId((cur) => cur || def.id);
          const url = def.lureUrl || def.lure_url;
          if (url) setLinkUrl((cur) => cur || url);
        }
      })
      .catch(() => setTargetLures([]));
    invoke("get_profile", { id: profileId })
      .then((p) => {
        const url = p?.lure_url || p?.lureUrl;
        if (url) setLinkUrl((cur) => cur || url);
      })
      .catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [profileId]);

  const applyPreset = (id) => {
    setPresetId(id);
    const p = presetById(id);
    if (!p) return;
    // Selecting a scenario resets to its vetted content and clears any saved
    // template selection (the content now comes from the preset).
    setName(p.label);
    setMode(p.mode);
    setRate(p.rate);
    setTemplateId("");
    setTemplateName(p.template.name);
    setSubject(p.template.subject);
    setHtml(p.template.html);
  };

  const pickTemplate = (id) => {
    setTemplateId(id);
    if (id) {
      const t = templates.find((x) => x.id === id);
      if (t) {
        setTemplateName(t.name);
        setSubject(t.subject);
        setHtml(templateHtml(t));
      }
    } else if (preset) {
      setTemplateName(preset.template.name);
      setSubject(preset.template.subject);
      setHtml(preset.template.html);
    }
  };

  const changeProfile = (id) => {
    setProfileId(id);
    // Explicit target change: reset lure/link so the effect refills defaults.
    setLureId("");
    setLinkUrl("");
  };

  const selectedList = lists.find((l) => l.id === listId);
  const recipientsOk = listId ? listCountOf(selectedList) > 0 || !selectedList : countEmails(emails) > 0;

  const canNext = useMemo(() => {
    switch (step) {
      case 0:
        return !!presetId;
      case 1:
        return !!profileId && !!linkUrl.trim();
      case 2:
        return !!subject.trim() && html.includes("{{link}}");
      case 3:
        return recipientsOk;
      case 4:
        return !!activeAccountId;
      default:
        return true;
    }
  }, [step, presetId, profileId, linkUrl, subject, html, recipientsOk, activeAccountId]);

  const selectedTarget = profiles.find((p) => p.id === profileId);

  const buildDraft = async () => {
    setBusy("guided");
    try {
      // Reuse the selected template (saving any edits back to it) or create a
      // new one from the scenario content.
      const t = await invoke("upsert_email_template", {
        req: {
          id: templateId || undefined,
          name: templateName || preset?.template.name || "Guided template",
          subject,
          htmlBody: html,
          assessmentId: assessmentId || null,
        },
      });

      // Reuse the chosen recipient list, or create one from pasted emails.
      let listIdToUse = listId;
      if (!listIdToUse) {
        const list = await invoke("create_recipient_list", {
          name: `${preset?.label || "guided"} recipients`,
          assessmentId: assessmentId || null,
        });
        await invoke("import_recipients_csv", { listId: list.id, csvText: emails });
        listIdToUse = list.id;
      }

      const c = await invoke("create_campaign", {
        req: {
          name: name || preset?.label || "campaign",
          templateId: t.id,
          listId: listIdToUse,
          linkUrl: linkUrl.trim(),
          profileId: profileId || undefined,
          assessmentId: assessmentId || undefined,
          lureId: lureId || undefined,
          senderAccountId: activeAccountId || undefined,
          ratePerMinute: rate,
          mode,
        },
      });
      // Remember the entities we just created/used so the pickers reflect them.
      setTemplateId(t.id);
      setListId(listIdToUse);
      setDraftId(c.id);
      setReview(await invoke("campaign_review", { campaignId: c.id }));
      loadLibrary().catch(() => {});
      append(`Draft “${c.name}” ready · ${c.pending} recipient(s)`);
    } catch (e) {
      append(String(e));
    } finally {
      setBusy("");
    }
  };

  const launch = async () => {
    if (!draftId) return;
    setBusy("guided");
    try {
      await invoke("start_campaign", { id: draftId });
      const launched = draftId;
      // Clear the persisted draft so the next visit starts fresh.
      skipPersistRef.current = true;
      clearDraft(storageKey);
      append("Campaign launched from Guided wizard");
      setDraftId("");
      setReview(null);
      onRefresh?.();
      onOpenResults?.(launched);
    } catch (e) {
      append(String(e));
      skipPersistRef.current = false;
    } finally {
      setBusy("");
    }
  };

  return (
    <div className="guided" data-testid="guided-view">
      <div className="composer-steps">
        {STEPS.map((label, i) => (
          <button
            key={label}
            type="button"
            data-testid={`guided-step-${i}`}
            className={`step ${i === step ? "active" : i < step ? "done" : ""}`}
            onClick={() => setStep(i)}
          >
            {i + 1} · {label}
          </button>
        ))}
      </div>

      {step === 0 && (
        <div className="wizard-body">
          <p className="muted">
            Pick a scenario. Each one ships a vetted email, a recommended phishlet pattern, and safe
            defaults — you can tweak everything later.
          </p>
          <div className="preset-grid">
            {SCENARIO_PRESETS.map((p) => (
              <button
                key={p.id}
                type="button"
                data-testid={`guided-preset-${p.id}`}
                className={`preset-card ${presetId === p.id ? "active" : ""}`}
                onClick={() => applyPreset(p.id)}
              >
                <strong>{p.label}</strong>
                <span className="tag small">{p.category}</span>
                <span>{p.blurb}</span>
              </button>
            ))}
          </div>
          {preset && <div className="guided-why">Why: {preset.why}</div>}
        </div>
      )}

      {step === 1 && (
        <div className="wizard-body">
          <div className="guided-why">
            {mode === "awareness"
              ? "Awareness mode never captures credentials — the link should point to a training or redirector page."
              : `Recommended phishlet pattern: ${preset?.phishletPattern || "any AiTM lure"}. Start the AiTM proxy on the target (Recon & Proxy) to generate a tracked lure.`}
          </div>
          <label className="block">
            Target
            <select value={profileId} onChange={(e) => changeProfile(e.target.value)}>
              <option value="">— choose a target —</option>
              {profiles.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name} {p.lure_url || p.lureUrl ? "· lure ready" : "· no lure yet"}
                </option>
              ))}
            </select>
          </label>
          {targetLures.length > 0 && (
            <label className="block">
              Lure
              <select
                value={lureId}
                onChange={(e) => {
                  const next = e.target.value;
                  setLureId(next);
                  const row = targetLures.find((r) => r.id === next);
                  if (row?.lureUrl || row?.lure_url) setLinkUrl(row.lureUrl || row.lure_url);
                }}
              >
                {targetLures.map((r) => (
                  <option key={r.id} value={r.id}>
                    {r.name}
                    {r.isDefault || r.is_default ? " (default)" : ""}
                  </option>
                ))}
              </select>
            </label>
          )}
          <label className="block">
            Link recipients will receive ({"{{link}}"})
            <input
              data-testid="guided-link"
              className="mono"
              value={linkUrl}
              onChange={(e) => setLinkUrl(e.target.value)}
              placeholder={
                mode === "awareness"
                  ? "https://training.example.com/oops"
                  : "tracked lure from Recon & Proxy"
              }
            />
          </label>
          {!linkUrl && selectedTarget && (
            <p className="muted small">
              No lure link yet. Open <strong>Recon &amp; Proxy</strong> on this target to start the
              AiTM proxy and generate one, or paste a URL above.
            </p>
          )}
        </div>
      )}

      {step === 2 && (
        <div className="wizard-body">
          <div className="guided-why">
            The message is pre-written for this scenario. Keep {"{{first_name}}"} and {"{{link}}"} so
            each recipient is personalized and tracked.
          </div>
          {templates.length > 0 && (
            <label className="block">
              Start from
              <select value={templateId} onChange={(e) => pickTemplate(e.target.value)}>
                <option value="">
                  Scenario default{preset ? ` (${preset.template.name})` : ""}
                </option>
                {templates.map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.name}
                  </option>
                ))}
              </select>
            </label>
          )}
          <div className="fields">
            <label>
              Template name
              <input value={templateName} onChange={(e) => setTemplateName(e.target.value)} />
            </label>
            <label>
              Subject
              <input value={subject} onChange={(e) => setSubject(e.target.value)} />
            </label>
          </div>
          <label className="block">
            Body (HTML)
            <textarea
              className="html-area"
              rows={8}
              value={html}
              onChange={(e) => setHtml(e.target.value)}
            />
          </label>
          {!html.includes("{{link}}") && (
            <div className="row">
              <span className="muted small">Missing {"{{link}}"} —</span>
              <button
                type="button"
                className="ghost"
                onClick={() =>
                  setHtml((h) => `${h}\n<p><a href="{{link}}">Continue</a></p>\n<p>{{link}}</p>`)
                }
              >
                Insert {"{{link}}"} CTA
              </button>
            </div>
          )}
          <div className="template-preview">
            <p className="muted small">Preview (sample merge, scripts stripped):</p>
            <iframe
              title="Guided preview"
              className="preview-frame"
              sandbox=""
              srcDoc={previewHtml(html).replace(
                /<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi,
                ""
              )}
            />
          </div>
        </div>
      )}

      {step === 3 && (
        <div className="wizard-body">
          <div className="guided-why">
            Reuse a saved recipient list, or paste new emails to create one. {preset?.recipientHint}{" "}
            Only send to recipients in your written scope.
          </div>
          {lists.length > 0 && (
            <label className="block">
              Recipient list
              <select value={listId} onChange={(e) => setListId(e.target.value)}>
                <option value="">— Paste new emails —</option>
                {lists.map((l) => (
                  <option key={l.id} value={l.id}>
                    {l.name} ({listCountOf(l)})
                  </option>
                ))}
              </select>
            </label>
          )}
          {listId ? (
            <p className="muted small">
              Using <strong>{selectedList?.name || "selected list"}</strong> ·{" "}
              {listCountOf(selectedList)} recipient(s).
            </p>
          ) : (
            <>
              <label className="block">
                Recipients
                <textarea
                  data-testid="guided-emails"
                  className="html-area"
                  rows={6}
                  value={emails}
                  onChange={(e) => setEmails(e.target.value)}
                  placeholder={"alice@client.com\nbob@client.com"}
                />
              </label>
              <p className="muted small">{countEmails(emails)} valid recipient(s) detected.</p>
            </>
          )}
        </div>
      )}

      {step === 4 && (
        <div className="wizard-body">
          <div className="guided-why">
            Choose which saved sender delivers this campaign. Add or edit senders in Settings →
            Delivery.
          </div>
          {accounts.length > 0 ? (
            <label className="block">
              Send from
              <select
                value={activeAccountId}
                onChange={(e) => setActiveAccountId(e.target.value)}
              >
                {accounts.map((a) => (
                  <option key={a.id} value={a.id}>
                    {a.label} — {a.fromEmail || a.from_email || a.provider}
                  </option>
                ))}
              </select>
            </label>
          ) : (
            <div className="ready-strip need">
              No sender configured — add one in <strong>Settings → Delivery</strong> first.
            </div>
          )}
          <button
            type="button"
            className="linkish"
            onClick={() => setAdvanced((v) => !v)}
          >
            {advanced ? "▾" : "▸"} Advanced (name, rate, mode)
          </button>
          {advanced && (
            <div className="advanced-block">
              <div className="fields">
                <label>
                  Campaign name
                  <input value={name} onChange={(e) => setName(e.target.value)} />
                </label>
                <label>
                  Rate / min
                  <input
                    type="number"
                    min={1}
                    max={600}
                    value={rate}
                    onChange={(e) => setRate(Number(e.target.value) || 10)}
                  />
                </label>
                <label>
                  Mode
                  <select value={mode} onChange={(e) => setMode(e.target.value)}>
                    <option value="aitm">AiTM capture (evilginx)</option>
                    <option value="awareness">Awareness (click-only)</option>
                  </select>
                </label>
              </div>
            </div>
          )}
        </div>
      )}

      {step === 5 && (
        <div className="wizard-body">
          <AupGate onAccepted={() => setAupOk(true)} />
          <ul className="review-summary">
            <li>
              <span className="lbl">Scenario</span>
              <span>{preset?.label}</span>
            </li>
            <li>
              <span className="lbl">Mode</span>
              <span>{mode === "awareness" ? "Awareness (no capture)" : "AiTM capture"}</span>
            </li>
            <li>
              <span className="lbl">Target</span>
              <span>{selectedTarget?.name || "—"}</span>
            </li>
            <li>
              <span className="lbl">Link</span>
              <span className="mono small truncate" title={linkUrl || ""}>
                {linkUrl || "—"}
              </span>
            </li>
            <li>
              <span className="lbl">Recipients</span>
              <span>{listId ? listCountOf(selectedList) : countEmails(emails)}</span>
            </li>
            <li>
              <span className="lbl">Sender</span>
              <span className="truncate" title={accounts.find((a) => a.id === activeAccountId)?.label || ""}>
                {accounts.find((a) => a.id === activeAccountId)?.label || "—"}
              </span>
            </li>
            <li>
              <span className="lbl">Rate</span>
              <span>{rate}/min</span>
            </li>
          </ul>

          {review && (
            <ul className="review-checks">
              {review.checks.map((c) => (
                <li key={c.id} className={c.ok ? "ok" : c.blocking ? "bad" : "warn"}>
                  <span className="mark">{c.ok ? "✓" : c.blocking ? "✕" : "!"}</span>
                  <span className="lbl">{c.label}</span>
                  <span className="muted small">{c.detail}</span>
                </li>
              ))}
            </ul>
          )}

          <div className="row">
            {!draftId ? (
              <button
                data-testid="guided-build"
                disabled={!!busy || !aupOk}
                onClick={buildDraft}
              >
                {busy === "guided" ? "Building…" : "Build draft & review"}
              </button>
            ) : (
              <>
                <button
                  data-testid="guided-launch"
                  disabled={!!busy || !review?.ready}
                  onClick={launch}
                >
                  {busy === "guided" ? "Launching…" : "Launch campaign"}
                </button>
                <button
                  className="ghost"
                  disabled={!!busy}
                  onClick={() => {
                    setDraftId("");
                    setReview(null);
                  }}
                >
                  Edit draft
                </button>
              </>
            )}
          </div>
        </div>
      )}

      <div className="wizard-nav">
        <button
          type="button"
          className="ghost"
          data-testid="guided-back"
          disabled={step === 0 || !!busy}
          onClick={() => setStep((s) => Math.max(0, s - 1))}
        >
          ← Back
        </button>
        {step < STEPS.length - 1 && (
          <button
            type="button"
            data-testid="guided-next"
            disabled={!canNext || !!busy}
            onClick={() => setStep((s) => Math.min(STEPS.length - 1, s + 1))}
          >
            Next →
          </button>
        )}
      </div>
    </div>
  );
}
