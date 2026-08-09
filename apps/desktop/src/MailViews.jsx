import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Hint, { LabelWithHint } from "./components/Hint";
import EmptyState from "./components/EmptyState";
import { IconMail, IconUsers, IconSend, IconChart } from "./lib/icons";
import AupGate from "./components/AupGate";
import GuidedCampaign from "./GuidedCampaign";
import {
  clearDraft as clearPersistedDraft,
  loadDraft as loadPersistedDraft,
  saveDraft as savePersistedDraft,
} from "./lib/draftState";
import {
  CONSUMER_SMTP_WARNING,
  DELIVERY_HINTS,
  GMAIL_NOTE,
  STARTER_TEMPLATE,
  isConsumerSmtpHost,
  isGmailPreset,
} from "./hints/delivery";
import { downloadText } from "./lib/download";

const PRESETS = [
  {
    id: "gmail",
    label: "Gmail (App Password)",
    blurb: "Fastest — email + 16-char app password",
  },
  { id: "ses_smtp", label: "Amazon SES", blurb: "SMTP for larger volume" },
  { id: "smtp", label: "Custom SMTP", blurb: "Any host / self-hosted" },
  { id: "resend", label: "Resend API", blurb: "BYO API key" },
  { id: "sendgrid", label: "SendGrid API", blurb: "BYO API key" },
  { id: "mailgun", label: "Mailgun API", blurb: "BYO API key" },
  { id: "postmark", label: "Postmark API", blurb: "BYO API key" },
];

function previewHtml(html) {
  const sample = {
    first_name: "Alex",
    email: "alex@example.com",
    link: "https://example.test/lure/preview",
  };
  return String(html || "")
    .replaceAll("{{first_name}}", sample.first_name)
    .replaceAll("{{email}}", sample.email)
    .replaceAll("{{link}}", sample.link);
}

/// Convert a datetime-local value ("YYYY-MM-DDTHH:MM") to the backend's
/// "YYYY-MM-DDTHH:MM:SSZ" UTC form.
function toIsoZ(local) {
  if (!local) return "";
  const d = new Date(local);
  if (Number.isNaN(d.getTime())) return "";
  return d.toISOString().replace(/\.\d{3}Z$/, "Z");
}

export function TemplatesView({ busy, setBusy, append, assessmentId = null }) {
  const [rows, setRows] = useState([]);
  const [id, setId] = useState("");
  const [name, setName] = useState("");
  const [subject, setSubject] = useState(STARTER_TEMPLATE.subject);
  const [html, setHtml] = useState(STARTER_TEMPLATE.html);
  const [importHelp, setImportHelp] = useState(false);
  const [editorTab, setEditorTab] = useState("source");

  const refresh = async () => {
    try {
      setRows(
        await invoke("list_email_templates", {
          assessmentId: assessmentId || null,
        })
      );
    } catch (e) {
      append(String(e));
    }
  };

  useEffect(() => {
    refresh();
  }, [assessmentId]);

  const applyImport = (r) => {
    setId("");
    setName(r.name || "Imported");
    setSubject(r.subject || "Imported email");
    setHtml(r.htmlBody ?? r.html_body ?? "");
    append(r.message || "Imported");
  };

  const importRaw = async (raw, filename) => {
    setBusy("import");
    try {
      const r = await invoke("import_email_source", {
        raw,
        filename: filename || null,
      });
      applyImport(r);
    } catch (e) {
      append(String(e));
    } finally {
      setBusy("");
    }
  };

  const onFile = async (file) => {
    if (!file) return;
    const text = await file.text();
    await importRaw(text, file.name);
  };

  const save = async () => {
    setBusy("template");
    try {
      const t = await invoke("upsert_email_template", {
        req: {
          id: id || undefined,
          name: name || "Untitled",
          subject,
          htmlBody: html,
          assessmentId: assessmentId || null,
        },
      });
      setId(t.id);
      append(`Saved template ${t.name}`);
      await refresh();
    } catch (e) {
      append(String(e));
    } finally {
      setBusy("");
    }
  };

  return (
    <section className="card" data-testid="templates-view">
      <div className="section-head">
        <h2 className="section-head-title">
          Email templates
          <Hint
            hint={`Import HTML from an existing email, then swap the CTA for {{link}}. Merge tags: {{first_name}}, {{email}}, {{link}}.${
              assessmentId
                ? " Scoped to this assessment, plus shared library templates."
                : ""
            }`}
          />
        </h2>
      </div>

      <div className="import-box">
        <div className="row">
          <label className="file-btn">
            Import .html / .eml
            <input
              type="file"
              accept=".html,.htm,.eml,text/html,message/rfc822"
              hidden
              onChange={(e) => {
                const f = e.target.files?.[0];
                onFile(f);
                e.target.value = "";
              }}
            />
          </label>
          <button
            type="button"
            className="ghost"
            disabled={!!busy || !html.trim()}
            onClick={() => importRaw(html, "paste.html")}
            title="Parse the HTML currently in the editor (or paste into it first)"
          >
            Parse pasted HTML
          </button>
          <button
            type="button"
            className="linkish"
            onClick={() => setImportHelp((v) => !v)}
          >
            {importHelp ? "▾" : "▸"} How to export from Gmail / Outlook
          </button>
        </div>
        {importHelp && (
          <pre className="hint-panel">{`Best options:

1) Save as .eml (recommended)
   • Gmail: open message → ⋮ → Download message (.eml)
   • Outlook: File → Save As → Outlook Message Format / .eml
   Then use “Import .html / .eml” above.

2) Copy HTML source
   • Gmail: ⋮ → Show original → copy, or use “Download original”
   • Or forward to yourself, open in browser, View Source, copy <html>…
   Paste into the HTML body box → “Parse pasted HTML”.

3) Design tools
   • Export HTML from Stripo / Beefree / MJML / Litmus, then import the .html file.

After import: replace the original button/URL with {{link}} and Save.`}</pre>
        )}
      </div>

      <div className="fields">
        <label>
          Name
          <input
            data-testid="template-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </label>
        <label>
          Subject
          <input
            data-testid="template-subject"
            value={subject}
            onChange={(e) => setSubject(e.target.value)}
          />
        </label>
      </div>
      <div className="editor-tabs" role="tablist" aria-label="Template editor">
        <button
          type="button"
          role="tab"
          data-testid="template-tab-source"
          aria-selected={editorTab === "source"}
          className={editorTab === "source" ? "active" : "ghost"}
          onClick={() => setEditorTab("source")}
        >
          Source
        </button>
        <button
          type="button"
          role="tab"
          data-testid="template-tab-preview"
          aria-selected={editorTab === "preview"}
          className={editorTab === "preview" ? "active" : "ghost"}
          onClick={() => setEditorTab("preview")}
        >
          Preview
        </button>
      </div>
      {editorTab === "source" ? (
      <label className="block">
        HTML body
        <textarea
          data-testid="template-html"
          className="html-area"
          rows={10}
          value={html}
          onChange={(e) => setHtml(e.target.value)}
          placeholder="Paste email HTML here, or import a .html / .eml file"
        />
      </label>
      ) : (
        <div className="template-preview">
          <p className="muted small">
            Sample merge: Alex / alex@example.com / preview lure link. Scripts are stripped for
            safety.
          </p>
          <iframe
            title="Template preview"
            className="preview-frame"
            sandbox=""
            srcDoc={previewHtml(html).replace(
              /<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi,
              ""
            )}
          />
        </div>
      )}
      <div className="row">
        <button
          className="ghost"
          type="button"
          data-testid="template-insert-link"
          disabled={!html}
          onClick={() => {
            if (!html.includes("{{link}}")) {
              setHtml(
                (h) =>
                  `${h}\n<p><a href="{{link}}">Continue</a></p>\n<p class="muted">{{link}}</p>`
              );
              append("Appended {{link}} CTA at the bottom — move it where you want");
            } else {
              append("Template already contains {{link}}");
            }
          }}
        >
          Insert {"{{link}}"} CTA
        </button>
        <button data-testid="template-save" disabled={!!busy} onClick={save}>
          Save template
        </button>
        <button
          className="ghost"
          data-testid="template-starter"
          disabled={!!busy}
          onClick={() => {
            setId("");
            setName(STARTER_TEMPLATE.name);
            setSubject(STARTER_TEMPLATE.subject);
            setHtml(STARTER_TEMPLATE.html);
            setEditorTab("source");
          }}
        >
          Load starter
        </button>
      </div>
      <ul className="list">
        {rows.map((t) => (
          <li key={t.id}>
            <span className="mono">{t.name}</span>
            <span className="tag">{t.subject}</span>
            <button
              className="ghost"
              onClick={() => {
                setId(t.id);
                setName(t.name);
                setSubject(t.subject);
                setHtml(t.htmlBody ?? t.html_body ?? "");
              }}
            >
              Edit
            </button>
            <button
              className="ghost"
              disabled={!!busy}
              onClick={async () => {
                await invoke("delete_email_template", { id: t.id });
                if (id === t.id) setId("");
                refresh();
              }}
            >
              Delete
            </button>
          </li>
        ))}
      </ul>
      {!rows.length && (
        <EmptyState compact icon={<IconMail size={20} />} title="No templates yet">
          Import a .html/.eml above, or load the starter to begin.
        </EmptyState>
      )}
    </section>
  );
}

function previewRecipientPaste(text, existingEmails = []) {
  const existing = new Set(
    existingEmails.map((e) => String(e || "").trim().toLowerCase()).filter(Boolean)
  );
  const seen = new Set();
  const valid = [];
  const invalid = [];
  const duplicates = [];
  const tokens = String(text || "")
    .split(/[\n,;]+/)
    .map((s) => s.trim())
    .filter(Boolean)
    .filter((s) => !/^email$/i.test(s));
  for (const raw of tokens) {
    const email = raw.includes("@")
      ? raw.replace(/^.*?</, "").replace(/>.*$/, "").trim().toLowerCase()
      : "";
    if (!email || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
      invalid.push(raw);
      continue;
    }
    if (seen.has(email) || existing.has(email)) {
      duplicates.push(email);
      continue;
    }
    seen.add(email);
    valid.push(email);
  }
  return { valid, invalid, duplicates };
}

export function RecipientsView({ busy, setBusy, append, assessmentId = null }) {
  const [lists, setLists] = useState([]);
  const [listId, setListId] = useState("");
  const [listName, setListName] = useState("targets");
  const [csv, setCsv] = useState("");
  const [recipients, setRecipients] = useState([]);

  const refresh = async () => {
    const ls = await invoke("list_recipient_lists", {
      assessmentId: assessmentId || null,
    });
    setLists(ls);
    if (!listId && ls[0]) setListId(ls[0].id);
  };

  useEffect(() => {
    refresh().catch((e) => append(String(e)));
  }, [assessmentId]);

  useEffect(() => {
    if (!listId) {
      setRecipients([]);
      return;
    }
    invoke("list_recipients", { listId })
      .then(setRecipients)
      .catch((e) => append(String(e)));
  }, [listId]);

  const importPreview = previewRecipientPaste(
    csv,
    recipients.map((r) => r.email)
  );

  const importPaste = async () => {
    setBusy("import");
    try {
      let lid = listId;
      if (!lid) {
        const l = await invoke("create_recipient_list", {
          name: listName || "targets",
          assessmentId: assessmentId || null,
        });
        lid = l.id;
        setListId(lid);
      }
      const r = await invoke("import_recipients_csv", {
        listId: lid,
        csvText: csv,
      });
      append(
        `Imported ${r.imported}, skipped ${r.skipped}` +
          (importPreview.duplicates.length
            ? ` · preview saw ${importPreview.duplicates.length} duplicate(s)`
            : "")
      );
      setRecipients(await invoke("list_recipients", { listId: lid }));
      await refresh();
      setCsv("");
    } catch (e) {
      append(String(e));
    } finally {
      setBusy("");
    }
  };

  return (
    <section className="card" data-testid="recipients-view">
      <div className="section-head">
        <h2 className="section-head-title">
          Recipients
          <Hint
            hint={`Recipient lists${
              assessmentId ? " for this assessment engagement" : ""
            }. Only send to recipients covered by your written authorization.`}
          />
        </h2>
      </div>
      <div className="fields">
        <label className="block">
          List name
          <input
            data-testid="recipient-list-name"
            value={listName}
            onChange={(e) => setListName(e.target.value)}
          />
        </label>
        <label className="block">
          Active list
          <select value={listId} onChange={(e) => setListId(e.target.value)}>
            <option value="">— new on import —</option>
            {lists.map((l) => (
              <option key={l.id} value={l.id}>
                {l.name} ({l.recipientCount ?? l.recipient_count})
              </option>
            ))}
          </select>
        </label>
      </div>
      <label className="block">
        <span className="label-with-hint">
          Paste emails
          <Hint hint="One email per line — no CSV header required. Or paste a CSV with an 'email' column (first/last optional)." />
        </span>
        <textarea
          data-testid="recipient-paste"
          className="html-area"
          rows={6}
          value={csv}
          onChange={(e) => setCsv(e.target.value)}
          placeholder={"alice@client.com\nbob@client.com"}
        />
      </label>
      {csv.trim() && (
        <p className="muted small">
          Preview: {importPreview.valid.length} new · {importPreview.duplicates.length}{" "}
          duplicate · {importPreview.invalid.length} invalid
          {importPreview.valid[0] ? ` · first ${importPreview.valid[0]}` : ""}
        </p>
      )}
      <div className="row">
        <button
          data-testid="recipient-import"
          disabled={!!busy || !csv.trim() || importPreview.valid.length === 0}
          onClick={importPaste}
        >
          Import into list
        </button>
        {listId && (
          <button
            className="ghost"
            disabled={!!busy}
            onClick={async () => {
              await invoke("delete_recipient_list", { id: listId });
              setListId("");
              refresh();
            }}
          >
            Delete list
          </button>
        )}
      </div>
      <table>
        <thead>
          <tr>
            <th>Email</th>
            <th>First</th>
            <th>Last</th>
          </tr>
        </thead>
        <tbody>
          {recipients.slice(0, 200).map((r) => (
            <tr key={r.id}>
              <td className="mono">{r.email}</td>
              <td>{r.firstName ?? r.first_name}</td>
              <td>{r.lastName ?? r.last_name}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {recipients.length > 200 && (
        <p className="muted">Showing first 200 of {recipients.length}.</p>
      )}
      {!recipients.length && (
        <EmptyState compact icon={<IconUsers size={20} />} title="No recipients yet">
          Paste emails above and import to build a list.
        </EmptyState>
      )}
    </section>
  );
}

function emptyMail() {
  return {
    provider: "gmail",
    host: "smtp.gmail.com",
    port: 587,
    username: "",
    password: "",
    fromEmail: "",
    fromName: "",
    useStarttls: true,
    apiKey: "",
    region: "us-east-1",
    domain: "",
  };
}

function fromApi(s) {
  return {
    provider: s.provider || "smtp",
    host: s.host || "",
    port: s.port || 587,
    username: s.username || "",
    password: s.password || "",
    fromEmail: s.fromEmail || s.from_email || "",
    fromName: s.fromName || s.from_name || "",
    useStarttls: s.useStarttls ?? s.use_starttls ?? true,
    apiKey: s.apiKey || s.api_key || "",
    region: s.region || "us-east-1",
    domain: s.domain || "",
  };
}

function applyPreset(prev, provider) {
  if (provider === "gmail") {
    return {
      ...prev,
      provider: "gmail",
      host: "smtp.gmail.com",
      port: 587,
      useStarttls: true,
      apiKey: "",
    };
  }
  if (provider === "ses_smtp") {
    return {
      ...prev,
      provider: "ses_smtp",
      host: `email-smtp.${prev.region || "us-east-1"}.amazonaws.com`,
      port: 587,
      useStarttls: true,
    };
  }
  return { ...prev, provider };
}

function accountToMail(a) {
  return {
    provider: a.provider || "gmail",
    host: a.host || "",
    port: a.port || 587,
    username: a.username || "",
    password: "", // don't echo; leave blank to keep existing on update
    fromEmail: a.fromEmail || a.from_email || "",
    fromName: a.fromName || a.from_name || "",
    useStarttls: a.useStarttls ?? a.use_starttls ?? true,
    apiKey: a.apiKey || a.api_key || "",
    region: a.region || "us-east-1",
    domain: a.domain || "",
  };
}

export function DeliverySettingsView({ busy, setBusy, append }) {
  const [mail, setMail] = useState(emptyMail());
  const [accountId, setAccountId] = useState("");
  const [label, setLabel] = useState("");
  const [accounts, setAccounts] = useState([]);
  const [testTo, setTestTo] = useState("");
  const [dnsOpen, setDnsOpen] = useState(false);

  const refreshAccounts = async () => {
    const list = await invoke("list_mail_accounts");
    setAccounts(list);
    const active = list.find((a) => a.active);
    if (active) {
      setAccountId(active.id);
      setLabel(active.label || "");
      setMail(accountToMail(active));
      if (active.fromEmail || active.from_email) {
        setTestTo(active.fromEmail || active.from_email);
      }
    } else if (!list.length) {
      setMail(emptyMail());
      setAccountId("");
      setLabel("");
    }
  };

  useEffect(() => {
    refreshAccounts().catch((e) => append(String(e)));
  }, []);

  const isGmail = mail.provider === "gmail";
  const isSmtp =
    mail.provider === "smtp" || mail.provider === "ses_smtp" || isGmail;
  const isHttp = !isSmtp;
  const consumer =
    isSmtp && !isGmail && isConsumerSmtpHost(mail.host);

  const save = async (asNew = false) => {
    setBusy("smtp");
    try {
      const fromEmail = (mail.fromEmail || mail.username || "").trim();
      const username =
        mail.provider === "gmail" ? fromEmail || mail.username : mail.username;
      const a = await invoke("upsert_mail_account", {
        req: {
          id: asNew ? undefined : accountId || undefined,
          label:
            label ||
            `${mail.provider} · ${fromEmail || "sender"}`,
          provider: mail.provider,
          host: mail.host,
          port: Number(mail.port) || 587,
          username,
          password: mail.password,
          fromEmail: fromEmail || username,
          fromName: mail.fromName,
          useStarttls: !!mail.useStarttls,
          apiKey: mail.apiKey,
          region: mail.region,
          domain: mail.domain,
          activate: true,
        },
      });
      setAccountId(a.id);
      setLabel(a.label);
      setMail(accountToMail(a));
      await refreshAccounts();
      append(`Saved sender “${a.label}” (${a.provider})`);
      return a;
    } catch (e) {
      append(String(e));
      throw e;
    } finally {
      setBusy("");
    }
  };

  return (
    <section className="card" data-testid="delivery-view">
      <div className="section-head">
        <h2 className="section-head-title">
          Delivery
          <Hint hint="Your global sender library. Save multiple senders (SMTP or ESP API); pick which one a campaign uses when you create it. Gmail needs a 16-char App Password." />
        </h2>
      </div>

      {accounts.length > 0 && (
        <div className="account-list">
          <h3>Saved senders</h3>
          <ul className="list">
            {accounts.map((a) => (
              <li key={a.id}>
                <span className="mono">
                  {a.label}
                  {a.active ? " · active" : ""}
                </span>
                <span className="tag">{a.provider}</span>
                <button
                  className="ghost"
                  disabled={!!busy}
                  onClick={() => {
                    setAccountId(a.id);
                    setLabel(a.label);
                    setMail(accountToMail(a));
                  }}
                >
                  Edit
                </button>
                {!a.active && (
                  <button
                    className="ghost"
                    disabled={!!busy}
                    onClick={async () => {
                      await invoke("activate_mail_account", { id: a.id });
                      append(`Active sender: ${a.label}`);
                      refreshAccounts();
                    }}
                  >
                    Use
                  </button>
                )}
                <button
                  className="ghost"
                  disabled={!!busy}
                  onClick={async () => {
                    await invoke("delete_mail_account", { id: a.id });
                    if (accountId === a.id) {
                      setAccountId("");
                      setLabel("");
                      setMail(emptyMail());
                    }
                    refreshAccounts();
                  }}
                >
                  Delete
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
          disabled={!!busy}
          onClick={() => {
            setAccountId("");
            setLabel("");
            setMail(emptyMail());
            setTestTo("");
          }}
        >
          + New sender
        </button>
      </div>

      <div className="preset-grid">
        {PRESETS.map((p) => (
          <button
            key={p.id}
            type="button"
            data-testid={`delivery-preset-${p.id}`}
            className={`preset-card ${mail.provider === p.id ? "active" : ""}`}
            onClick={() => setMail((m) => applyPreset(m, p.id))}
          >
            <strong>{p.label}</strong>
            <span>{p.blurb}</span>
          </button>
        ))}
      </div>

      {isGmail && <div className="info-banner">{GMAIL_NOTE}</div>}
      {isHttp && <div className="warn-banner">{DELIVERY_HINTS.espAdapter.body}</div>}
      {consumer && <div className="warn-banner">{CONSUMER_SMTP_WARNING}</div>}

      <label className="block">
        Label (how this sender appears in the list)
        <input
          data-testid="delivery-label"
          value={label}
          onChange={(e) => setLabel(e.target.value)}
          placeholder={
            mail.fromEmail
              ? `${mail.provider} · ${mail.fromEmail}`
              : "e.g. Client A Gmail"
          }
        />
      </label>

      {isGmail && (
        <>
          <div className="fields">
            <label>
              <LabelWithHint hint={DELIVERY_HINTS.gmail}>Gmail address</LabelWithHint>
              <input
                value={mail.fromEmail || mail.username}
                onChange={(e) => {
                  const v = e.target.value;
                  setMail({ ...mail, fromEmail: v, username: v });
                  if (!testTo) setTestTo(v);
                }}
                placeholder="you@gmail.com"
                autoComplete="username"
              />
            </label>
            <label>
              App Password
              <input
                type="password"
                value={mail.password}
                onChange={(e) => setMail({ ...mail, password: e.target.value })}
                placeholder={
                  accountId ? "leave blank to keep saved" : "xxxx xxxx xxxx xxxx"
                }
                autoComplete="current-password"
              />
            </label>
          </div>
          <label className="block">
            From name (optional)
            <input
              value={mail.fromName}
              onChange={(e) => setMail({ ...mail, fromName: e.target.value })}
              placeholder="Security Team"
            />
          </label>
          <p className="muted small mono">
            smtp.gmail.com:587 · STARTTLS · username = your Gmail
          </p>
        </>
      )}

      {mail.provider === "ses_smtp" && (
        <div className="fields">
          <label>
            AWS region
            <input
              value={mail.region}
              onChange={(e) => {
                const region = e.target.value;
                setMail({
                  ...mail,
                  region,
                  host: `email-smtp.${region || "us-east-1"}.amazonaws.com`,
                });
              }}
              placeholder="us-east-1"
            />
          </label>
          <label>
            SMTP username
            <input
              value={mail.username}
              onChange={(e) => setMail({ ...mail, username: e.target.value })}
            />
          </label>
          <label>
            SMTP password
            <input
              type="password"
              value={mail.password}
              onChange={(e) => setMail({ ...mail, password: e.target.value })}
            />
          </label>
          <label>
            From email
            <input
              value={mail.fromEmail}
              onChange={(e) => setMail({ ...mail, fromEmail: e.target.value })}
            />
          </label>
        </div>
      )}

      {mail.provider === "smtp" && (
        <>
          <div className="fields">
            <label>
              <LabelWithHint hint={DELIVERY_HINTS.smtpHost}>SMTP host</LabelWithHint>
              <input
                value={mail.host}
                onChange={(e) => setMail({ ...mail, host: e.target.value })}
                placeholder="mail.example.com"
              />
            </label>
            <label>
              Port
              <input
                type="number"
                value={mail.port}
                onChange={(e) => setMail({ ...mail, port: e.target.value })}
              />
            </label>
            <label>
              Username
              <input
                value={mail.username}
                onChange={(e) => setMail({ ...mail, username: e.target.value })}
              />
            </label>
            <label>
              Password
              <input
                type="password"
                value={mail.password}
                onChange={(e) => setMail({ ...mail, password: e.target.value })}
              />
            </label>
          </div>
          <div className="fields">
            <label>
              From email
              <input
                value={mail.fromEmail}
                onChange={(e) => setMail({ ...mail, fromEmail: e.target.value })}
              />
            </label>
            <label>
              From name
              <input
                value={mail.fromName}
                onChange={(e) => setMail({ ...mail, fromName: e.target.value })}
              />
            </label>
          </div>
          <label className="check">
            <input
              type="checkbox"
              checked={!!mail.useStarttls}
              onChange={(e) => setMail({ ...mail, useStarttls: e.target.checked })}
            />
            STARTTLS
          </label>
        </>
      )}

      {isHttp && (
        <div className="fields">
          <label>
            <LabelWithHint hint={DELIVERY_HINTS.espAdapter}>API key</LabelWithHint>
            <input
              type="password"
              value={mail.apiKey}
              onChange={(e) => setMail({ ...mail, apiKey: e.target.value })}
            />
          </label>
          <label>
            From email
            <input
              value={mail.fromEmail}
              onChange={(e) => setMail({ ...mail, fromEmail: e.target.value })}
            />
          </label>
          {mail.provider === "mailgun" && (
            <>
              <label>
                Domain
                <input
                  value={mail.domain}
                  onChange={(e) => setMail({ ...mail, domain: e.target.value })}
                />
              </label>
              <label>
                Region
                <select
                  value={mail.region === "eu" ? "eu" : "us"}
                  onChange={(e) => setMail({ ...mail, region: e.target.value })}
                >
                  <option value="us">US</option>
                  <option value="eu">EU</option>
                </select>
              </label>
            </>
          )}
        </div>
      )}

      <div className="row dns-toggle">
        <button type="button" className="linkish" onClick={() => setDnsOpen((v) => !v)}>
          {dnsOpen ? "▾" : "▸"} DNS checklist
        </button>
        <Hint hint={DELIVERY_HINTS.dnsAuth} />
      </div>
      {dnsOpen && <pre className="hint-panel">{DELIVERY_HINTS.dnsAuth.body}</pre>}

      <div className="row">
        <button
          data-testid="delivery-save"
          disabled={!!busy}
          onClick={() => save(false).catch(() => {})}
        >
          {accountId ? "Update & use" : "Save & use"}
        </button>
        {accountId && (
          <button
            type="button"
            className="ghost"
            disabled={!!busy}
            onClick={() => save(true).catch(() => {})}
          >
            Save as new sender
          </button>
        )}
        <label>
          Test to
          <input
            data-testid="delivery-test-to"
            value={testTo}
            onChange={(e) => setTestTo(e.target.value)}
            placeholder={mail.fromEmail || "you@gmail.com"}
          />
        </label>
        <button
          data-testid="delivery-send-test"
          disabled={!!busy || !(testTo || mail.fromEmail)}
          onClick={async () => {
            setBusy("test");
            try {
              await save(false);
              const to = testTo || mail.fromEmail;
              const r = await invoke("send_test_email", { to });
              append(`Test sent to ${r.to}`);
            } catch (e) {
              append(String(e));
            } finally {
              setBusy("");
            }
          }}
        >
          Save & send test
        </button>
      </div>
    </section>
  );
}

export function CampaignsView({
  showAdvancedFlows = true,
  busy,
  setBusy,
  append,
  prefill,
  onOpenResults,
  assessmentId = null,
  activeTargetId = null,
}) {
  // Persist in-progress composer/express state per assessment so switching
  // tabs (which unmounts this view) and back doesn't discard the draft.
  const storageKey = `composer.${assessmentId || "global"}`;
  const savedRef = useRef(null);
  if (savedRef.current === null) savedRef.current = loadPersistedDraft(storageKey);
  const saved = savedRef.current;
  const skipPersistRef = useRef(false);

  const [templates, setTemplates] = useState([]);
  const [lists, setLists] = useState([]);
  const [profiles, setProfiles] = useState([]);
  const [campaigns, setCampaigns] = useState([]);
  const [accounts, setAccounts] = useState([]);
  const [activeAccountId, setActiveAccountId] = useState(() => saved.activeAccountId ?? "");
  const [name, setName] = useState(() => saved.name ?? "campaign");
  const [templateId, setTemplateId] = useState(() => saved.templateId ?? "");
  const [listId, setListId] = useState(() => saved.listId ?? "");
  const [linkUrl, setLinkUrl] = useState(() => saved.linkUrl ?? "");
  const [profileId, setProfileId] = useState(() => saved.profileId ?? "");
  const [rate, setRate] = useState(() => saved.rate ?? 10);
  const [aupOk, setAupOk] = useState(false);
  const [delivery, setDelivery] = useState(null);
  const [quickEmails, setQuickEmails] = useState(() => saved.quickEmails ?? "");
  const [showAdvanced, setShowAdvanced] = useState(() => saved.showAdvanced ?? false);
  const [lureId, setLureId] = useState(() => saved.lureId ?? "");
  const [targetLures, setTargetLures] = useState([]);
  const [flow, setFlow] = useState(() => saved.flow ?? "guided");
  useEffect(() => {
    if (!showAdvancedFlows && flow !== "guided") setFlow("guided");
  }, [showAdvancedFlows, flow]);
  const [draftId, setDraftId] = useState(() => saved.draftId ?? "");
  const [review, setReview] = useState(null);
  const [testTo, setTestTo] = useState(() => saved.testTo ?? "");
  const [campaignMode, setCampaignMode] = useState(() => saved.campaignMode ?? "aitm");
  const [scheduledAt, setScheduledAt] = useState(() => saved.scheduledAt ?? "");
  const [windowStart, setWindowStart] = useState(() => saved.windowStart ?? "");
  const [windowEnd, setWindowEnd] = useState(() => saved.windowEnd ?? "");

  useEffect(() => {
    if (skipPersistRef.current) return;
    savePersistedDraft(storageKey, {
      activeAccountId,
      name,
      templateId,
      listId,
      linkUrl,
      profileId,
      rate,
      quickEmails,
      showAdvanced,
      lureId,
      flow,
      draftId,
      testTo,
      campaignMode,
      scheduledAt,
      windowStart,
      windowEnd,
    });
  }, [
    storageKey,
    activeAccountId,
    name,
    templateId,
    listId,
    linkUrl,
    profileId,
    rate,
    quickEmails,
    showAdvanced,
    lureId,
    flow,
    draftId,
    testTo,
    campaignMode,
    scheduledAt,
    windowStart,
    windowEnd,
  ]);

  const refresh = async () => {
    const profilesPromise = assessmentId
      ? invoke("cmd_list_targets", { assessmentId })
      : invoke("list_profiles");
    const [t, l, p, c, d, acc] = await Promise.all([
      invoke("list_email_templates", { assessmentId: assessmentId || null }),
      invoke("list_recipient_lists", { assessmentId: assessmentId || null }),
      profilesPromise,
      invoke("list_campaigns", { assessmentId: assessmentId || null }),
      invoke("get_smtp_settings"),
      invoke("list_mail_accounts").catch(() => []),
    ]);
    setTemplates(t);
    setLists(l);
    setProfiles(p);
    setCampaigns(c);
    setDelivery(fromApi(d));
    setAccounts(acc);
    const active = acc.find((a) => a.active);
    setActiveAccountId((cur) => cur || active?.id || "");
    if (isGmailPreset(d.provider, d.host)) {
      setRate((r) => (r > 15 ? 10 : r));
    }
    if (!templateId && t[0]) setTemplateId(t[0].id);
    if (!listId && l[0]) setListId(l[0].id);
    if (!profileId && !prefill?.profileId) {
      const preferId = activeTargetId || null;
      const pick =
        (preferId && p.find((x) => x.id === preferId)) ||
        p.find((x) => x.lure_url || x.lureUrl);
      if (pick) {
        setProfileId(pick.id);
        if (!linkUrl) setLinkUrl(pick.lure_url || pick.lureUrl || "");
      }
    }
  };

  useEffect(() => {
    refresh().catch((e) => append(String(e)));
    invoke("get_aup_status")
      .then((s) => setAupOk(!!s.accepted))
      .catch(() => {});
    const t = setInterval(() => {
      invoke("list_campaigns", { assessmentId: assessmentId || null })
        .then(setCampaigns)
        .catch(() => {});
    }, 2000);
    return () => clearInterval(t);
  }, [assessmentId, activeTargetId]);

  useEffect(() => {
    if (!prefill) return;
    if (prefill.linkUrl) setLinkUrl(prefill.linkUrl);
    if (prefill.profileId) setProfileId(prefill.profileId);
  }, [prefill]);

  useEffect(() => {
    if (!activeTargetId || prefill?.profileId) return;
    setProfileId(activeTargetId);
  }, [activeTargetId, prefill?.profileId]);

  useEffect(() => {
    if (!profileId) {
      setTargetLures([]);
      setLureId("");
      return;
    }
    invoke("get_profile", { id: profileId })
      .then((p) => {
        // Don't clobber a restored/edited link; only fill when empty.
        const url = p?.lure_url || p?.lureUrl;
        if (url) setLinkUrl((cur) => cur || url);
      })
      .catch(() => {});
    invoke("cmd_list_lures", { profileId })
      .then((rows) => {
        setTargetLures(rows);
        const def = rows.find((r) => r.isDefault || r.is_default) || rows[0];
        if (def) {
          setLureId((cur) => cur || def.id);
          const url = def.lureUrl || def.lure_url;
          if (url) setLinkUrl((cur) => cur || url);
        }
      })
      .catch(() => {
        setTargetLures([]);
      });
  }, [profileId]);

  // A restored draft keeps its campaign id but not the server-side review;
  // re-fetch it so the Launch action is enabled without rebuilding.
  useEffect(() => {
    if (draftId && !review) {
      invoke("campaign_review", { campaignId: draftId })
        .then(setReview)
        .catch(() => {});
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [draftId]);

  const deliveryReady = !!(
    delivery &&
    (delivery.password || delivery.apiKey) &&
    (delivery.fromEmail || delivery.username)
  );

  const ensureTemplate = async () => {
    if (templateId) return templateId;
    const t = await invoke("upsert_email_template", {
      req: {
        name: STARTER_TEMPLATE.name,
        subject: STARTER_TEMPLATE.subject,
        htmlBody: STARTER_TEMPLATE.html,
        assessmentId: assessmentId || null,
      },
    });
    setTemplateId(t.id);
    setTemplates(
      await invoke("list_email_templates", {
        assessmentId: assessmentId || null,
      })
    );
    return t.id;
  };

  const ensureRecipients = async () => {
    if (listId) {
      const count =
        lists.find((l) => l.id === listId)?.recipientCount ??
        lists.find((l) => l.id === listId)?.recipient_count ??
        0;
      if (count > 0) return listId;
    }
    if (!quickEmails.trim()) {
      throw new Error("Paste at least one recipient email below");
    }
    const l = await invoke("create_recipient_list", {
      name: "quick-send",
      assessmentId: assessmentId || null,
    });
    await invoke("import_recipients_csv", {
      listId: l.id,
      csvText: quickEmails,
    });
    setListId(l.id);
    setLists(
      await invoke("list_recipient_lists", {
        assessmentId: assessmentId || null,
      })
    );
    setQuickEmails("");
    return l.id;
  };

  const quickSend = async () => {
    setBusy("campaign");
    try {
      if (!deliveryReady) {
        throw new Error("No sender configured — add one in Settings → Delivery");
      }
      const tid = await ensureTemplate();
      const lid = await ensureRecipients();
      if (!linkUrl.trim()) {
        throw new Error("Need a tracked link — start a destination proxy or paste a URL");
      }
      const c = await invoke("create_campaign", {
        req: {
          name: name || "campaign",
          templateId: tid,
          listId: lid,
          linkUrl: linkUrl.trim(),
          profileId: profileId || undefined,
          assessmentId: assessmentId || undefined,
          lureId: lureId || undefined,
          senderAccountId: activeAccountId || undefined,
          ratePerMinute: rate,
        },
      });
      append(
        `Sending ${c.pending} · ETA ~${Math.ceil((c.etaSeconds ?? c.eta_seconds ?? 0) / 60)}m`
      );
      await invoke("start_campaign", { id: c.id });
      await refresh();
    } catch (e) {
      append(String(e));
    } finally {
      setBusy("");
    }
  };

  const buildBaseReq = (tid, lid) => ({
    name: name || "campaign",
    templateId: tid,
    listId: lid,
    linkUrl: linkUrl.trim(),
    profileId: profileId || undefined,
    assessmentId: assessmentId || undefined,
    lureId: lureId || undefined,
    senderAccountId: activeAccountId || undefined,
    ratePerMinute: rate,
    mode: campaignMode,
    scheduledAt: toIsoZ(scheduledAt) || undefined,
    sendWindowStart: windowStart || undefined,
    sendWindowEnd: windowEnd || undefined,
  });

  const runReview = async (cid) => {
    try {
      setReview(await invoke("campaign_review", { campaignId: cid }));
    } catch (e) {
      append(String(e));
      setReview(null);
    }
  };

  // Composer: create a draft (with sender + content snapshot) without sending.
  const saveDraft = async () => {
    setBusy("campaign");
    try {
      if (!templateId) {
        throw new Error("Choose a template below — the composer won't auto-create one");
      }
      const lid = await ensureRecipients();
      if (!linkUrl.trim()) {
        throw new Error("Add a tracked link (AiTM) or awareness URL");
      }
      const c = await invoke("create_campaign", { req: buildBaseReq(templateId, lid) });
      setDraftId(c.id);
      setName(c.name);
      if (!testTo && (delivery?.fromEmail || delivery?.username)) {
        setTestTo(delivery.fromEmail || delivery.username);
      }
      append(`Draft “${c.name}” created · ${c.pending} recipient(s)`);
      await runReview(c.id);
      await refresh();
    } catch (e) {
      append(String(e));
    } finally {
      setBusy("");
    }
  };

  const sendTest = async () => {
    if (!draftId) return;
    setBusy("test");
    try {
      const r = await invoke("send_campaign_test", { campaignId: draftId, to: testTo });
      append(`Test sent to ${r.to}`);
    } catch (e) {
      append(String(e));
    } finally {
      setBusy("");
    }
  };

  const launchDraft = async () => {
    if (!draftId) return;
    setBusy("campaign");
    try {
      await invoke("start_campaign", { id: draftId });
      const launched = draftId;
      // Clear the persisted draft so returning to Campaigns starts fresh.
      skipPersistRef.current = true;
      clearPersistedDraft(storageKey);
      append("Campaign launched");
      setDraftId("");
      setReview(null);
      await refresh();
      if (onOpenResults) onOpenResults(launched);
    } catch (e) {
      append(String(e));
      skipPersistRef.current = false;
    } finally {
      setBusy("");
    }
  };

  const discardDraft = () => {
    setDraftId("");
    setReview(null);
  };

  return (
    <section className="card" data-testid="campaigns-view">
      <div className="section-head">
        <h2 className="section-head-title">
          Campaigns
          <Hint hint="Three ways to build a send — Guided (scenario wizard, best for non-technical operators), Composer (draft → review → test → launch, with content & sender snapshotted at draft time), and Express (quick pick-and-send)." />
        </h2>
      </div>

      <div className="flow-toggle" role="tablist" aria-label="Campaign flow">
        <button
          type="button"
          role="tab"
          data-testid="campaign-flow-guided"
          aria-selected={flow === "guided"}
          className={flow === "guided" ? "active" : "ghost"}
          onClick={() => {
            discardDraft();
            setFlow("guided");
          }}
        >
          Guided
        </button>
        {showAdvancedFlows && (
          <>
            <button
              type="button"
              role="tab"
              data-testid="campaign-flow-composer"
              aria-selected={flow === "composer"}
              className={flow === "composer" ? "active" : "ghost"}
              onClick={() => setFlow("composer")}
            >
              Composer
            </button>
            <button
              type="button"
              role="tab"
              data-testid="campaign-flow-express"
              aria-selected={flow === "express"}
              className={flow === "express" ? "active" : "ghost"}
              onClick={() => {
                discardDraft();
                setFlow("express");
              }}
            >
              Express
            </button>
          </>
        )}
      </div>
      <p className="muted small flow-desc">
        {flow === "guided"
          ? "Pick a scenario with safe defaults and follow the steps."
          : flow === "composer"
          ? "Compose, review, send yourself a test, then launch."
          : "Pick a saved sender, paste recipients, set a tracked link, send."}
      </p>

      {flow === "guided" ? (
        <GuidedCampaign
          assessmentId={assessmentId}
          activeTargetId={activeTargetId}
          append={append}
          busy={busy}
          setBusy={setBusy}
          onOpenResults={onOpenResults}
          onRefresh={refresh}
        />
      ) : (
        <>
      <AupGate onAccepted={() => setAupOk(true)} />

      {accounts.length > 0 ? (
        <label className="block">
          <span className="label-with-hint">
            Send from
            <Hint hint="Chosen per campaign and snapshotted when you create it — this does not change the global default sender in Settings → Delivery." />
          </span>
          <select
            value={activeAccountId}
            disabled={!!busy}
            onChange={async (e) => {
              const id = e.target.value;
              setActiveAccountId(id);
              // Per-campaign pick only: refresh the readiness snapshot for this
              // sender WITHOUT calling activate_mail_account (which would change
              // the global default). The chosen id rides along in the campaign req.
              try {
                const d =
                  (await invoke("get_settings_for_account", { id })) ||
                  (await invoke("get_smtp_settings"));
                setDelivery(fromApi(d));
                const picked = accounts.find((a) => a.id === id);
                if (picked) append(`Send from: ${picked.label}`);
              } catch (err) {
                append(String(err));
              }
            }}
          >
            {accounts.map((a) => (
              <option key={a.id} value={a.id}>
                {a.label} — {a.fromEmail || a.from_email || a.provider}
              </option>
            ))}
          </select>
          <span className="muted small">Add or edit senders in Settings → Delivery.</span>
        </label>
      ) : (
        <div className="ready-strip need">
          No sender yet — add one in <strong>Settings → Delivery</strong>, then send from here.
        </div>
      )}

      <label className="block">
        Recipients (paste emails — skip if a list is already selected)
        <textarea
          className="html-area"
          rows={3}
          value={quickEmails}
          onChange={(e) => setQuickEmails(e.target.value)}
          placeholder={"alice@client.com\nbob@client.com"}
        />
      </label>

      <div className="fields">
        <label>
          Template
          <select value={templateId} onChange={(e) => setTemplateId(e.target.value)}>
            <option value="">
              {flow === "composer" ? "— choose a template —" : "— auto starter —"}
            </option>
            {templates.map((t) => (
              <option key={t.id} value={t.id}>
                {t.name}
              </option>
            ))}
          </select>
        </label>
        <label>
          List (optional)
          <select value={listId} onChange={(e) => setListId(e.target.value)}>
            <option value="">— create from paste —</option>
            {lists.map((l) => (
              <option key={l.id} value={l.id}>
                {l.name} ({l.recipientCount ?? l.recipient_count})
              </option>
            ))}
          </select>
        </label>
        {!assessmentId && (
        <label>
          Destination
          <select
            value={profileId}
            onChange={(e) => {
              setProfileId(e.target.value);
              setLureId("");
              setLinkUrl("");
            }}
          >
            <option value="">—</option>
            {profiles.map((p) => (
              <option key={p.id} value={p.id}>
                  {p.name} {p.lure_url || p.lureUrl ? "· link" : ""}
              </option>
            ))}
          </select>
        </label>
        )}
      </div>
      {assessmentId && profileId && (
        <p className="muted small">
          Target: {profiles.find((p) => p.id === profileId)?.name || profileId}
        </p>
      )}
      {targetLures.length > 0 && (
        <label className="block">
          Lure
          <select
            value={lureId}
            onChange={(e) => {
              const next = e.target.value;
              setLureId(next);
              const row = targetLures.find((r) => r.id === next);
              if (row?.lureUrl || row?.lure_url) {
                setLinkUrl(row.lureUrl || row.lure_url);
              }
            }}
          >
            {targetLures.map((r) => (
              <option key={r.id} value={r.id}>
                {r.name}
                {r.isDefault || r.is_default ? " (default)" : ""}
                {r.lureUrl || r.lure_url
                  ? ` · ${String(r.lureUrl || r.lure_url).slice(0, 48)}`
                  : ""}
              </option>
            ))}
          </select>
        </label>
      )}

      <label className="block">
        <LabelWithHint hint={DELIVERY_HINTS.linkUrl}>Tracked link</LabelWithHint>
        <input
          className="mono"
          value={linkUrl}
          onChange={(e) => setLinkUrl(e.target.value)}
          placeholder="from Destinations → Use in Campaigns"
        />
      </label>

      <button
        type="button"
        className="linkish"
        onClick={() => setShowAdvanced((v) => !v)}
      >
        {showAdvanced ? "▾" : "▸"} Advanced (name, rate, mode, schedule, send window)
      </button>
      {showAdvanced && (
        <div className="advanced-block">
        <div className="fields" style={{ marginTop: 8 }}>
          <label>
            Name
            <input value={name} onChange={(e) => setName(e.target.value)} />
          </label>
          <label>
            <LabelWithHint hint={DELIVERY_HINTS.rateLimit}>Rate / min</LabelWithHint>
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
              <select value={campaignMode} onChange={(e) => setCampaignMode(e.target.value)}>
                <option value="aitm">AiTM capture (evilginx)</option>
                <option value="awareness">Awareness (click-only training)</option>
              </select>
            </label>
          </div>
          <div className="fields">
            <label>
              Launch at (optional)
              <input
                type="datetime-local"
                value={scheduledAt}
                onChange={(e) => setScheduledAt(e.target.value)}
              />
            </label>
            <label>
              Send window start (UTC)
              <input
                type="time"
                value={windowStart}
                onChange={(e) => setWindowStart(e.target.value)}
              />
            </label>
            <label>
              Send window end (UTC)
              <input
                type="time"
                value={windowEnd}
                onChange={(e) => setWindowEnd(e.target.value)}
              />
            </label>
          </div>
          {campaignMode === "awareness" && (
            <p className="muted small">
              Awareness mode never captures credentials — the link should point to a training or
              redirector page for pure click metrics.
            </p>
          )}
        </div>
      )}

      {flow === "express" ? (
      <div className="row" style={{ marginTop: 12 }}>
          <button disabled={!!busy || !aupOk || !linkUrl} onClick={quickSend}>
          {busy === "campaign" ? "Sending…" : "Send campaign"}
        </button>
        <button className="ghost" disabled={!!busy} onClick={() => refresh()}>
          Refresh
        </button>
      </div>
      ) : draftId ? (
        <div className="composer-panel">
          <div className="composer-steps">
            <span className="step done">1 · Draft</span>
            <span className={`step ${review?.ready ? "done" : "active"}`}>2 · Review</span>
            <span className="step">3 · Test</span>
            <span className="step">4 · Launch</span>
          </div>
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
            <label>
              Test to
              <input
                value={testTo}
                onChange={(e) => setTestTo(e.target.value)}
                placeholder="you@company.com"
              />
            </label>
            <button className="ghost" disabled={!!busy || !testTo} onClick={sendTest}>
              {busy === "test" ? "Sending…" : "Send test"}
            </button>
            <button className="ghost" disabled={!!busy} onClick={() => runReview(draftId)}>
              Re-run review
            </button>
          </div>
          <div className="row">
            <button disabled={!!busy || !review?.ready} onClick={launchDraft}>
              {busy === "campaign"
                ? "Launching…"
                : scheduledAt
                ? "Schedule & launch"
                : "Launch campaign"}
            </button>
            <button className="ghost" disabled={!!busy} onClick={discardDraft}>
              Discard draft
            </button>
          </div>
        </div>
      ) : (
        <div className="row" style={{ marginTop: 12 }}>
          <button disabled={!!busy || !aupOk || !linkUrl || !templateId} onClick={saveDraft}>
            {busy === "campaign" ? "Saving…" : "Save draft"}
          </button>
          <button className="ghost" disabled={!!busy} onClick={() => refresh()}>
            Refresh
          </button>
          {!templateId && (
            <span className="muted small">Choose a template to enable Save draft.</span>
          )}
        </div>
      )}
        </>
      )}

      <table>
        <thead>
          <tr>
            <th>Name</th>
            <th>Status</th>
            <th>Progress</th>
            <th>Sent</th>
            <th>Failed</th>
            <th>Pending</th>
            <th>ETA</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {campaigns.map((c) => (
            <tr key={c.id}>
              <td>{c.name}</td>
              <td>
                {c.status}
                {c.mode === "awareness" && <span className="tag small"> awareness</span>}
                {c.status === "scheduled" && (c.scheduledAt || c.scheduled_at) && (
                  <span className="muted small block">
                    {new Date(c.scheduledAt || c.scheduled_at).toLocaleString()}
                  </span>
                )}
              </td>
              <td>
                <div className="progress-wrap" title={`${Math.round(c.progressPct ?? c.progress_pct ?? 0)}%`}>
                  <div
                    className="progress-bar"
                    style={{ width: `${Math.min(100, c.progressPct ?? c.progress_pct ?? 0)}%` }}
                  />
                </div>
                <span className="small muted">
                  {Math.round(c.progressPct ?? c.progress_pct ?? 0)}%
                  {c.total ? ` · ${c.total}` : ""}
                </span>
              </td>
              <td>{c.sent}</td>
              <td>{c.failed}</td>
              <td>{c.pending}</td>
              <td className="small">
                {c.status === "running" && (c.etaSeconds ?? c.eta_seconds)
                  ? `~${Math.ceil((c.etaSeconds ?? c.eta_seconds) / 60)}m`
                  : "—"}
              </td>
              <td className="row">
                {(c.status === "draft" || c.status === "paused" || (c.pending > 0 && c.status !== "running")) && (
                  <button
                    className="ghost"
                    disabled={!!busy}
                    onClick={() =>
                      invoke("start_campaign", { id: c.id }).then(refresh).catch((e) => append(String(e)))
                    }
                  >
                    {c.status === "draft"
                      ? "Start"
                      : c.status === "scheduled"
                      ? "Launch"
                      : "Resume"}
                  </button>
                )}
                {c.status === "running" && (
                  <button
                    className="ghost"
                    disabled={!!busy}
                    onClick={() =>
                      invoke("stop_campaign", { id: c.id }).then(refresh).catch((e) => append(String(e)))
                    }
                  >
                    Pause
                  </button>
                )}
                {c.failed > 0 && c.status !== "running" && (
                  <button
                    className="ghost"
                    disabled={!!busy}
                    onClick={() =>
                      invoke("retry_failed_campaign", { id: c.id })
                        .then(() => invoke("start_campaign", { id: c.id }))
                        .then(refresh)
                        .catch((e) => append(String(e)))
                    }
                  >
                    Retry failed
                  </button>
                )}
                {onOpenResults && (
                  <button
                    className="ghost"
                    type="button"
                    onClick={() => onOpenResults(c.id)}
                  >
                    Results
                  </button>
                )}
                {c.status !== "running" && (
                  <button
                    className="ghost danger"
                    type="button"
                    disabled={!!busy}
                    title="Permanently delete this campaign and its attempt/tracking rows"
                    onClick={() => {
                      if (
                        window.confirm(
                          `Delete campaign “${c.name}”?\n\nThis removes the campaign and all of its attempt/tracking rows. This cannot be undone.`
                        )
                      ) {
                        invoke("delete_campaign", { id: c.id })
                          .then(() => append(`Deleted “${c.name}”`))
                          .then(refresh)
                          .catch((e) => append(String(e)));
                      }
                    }}
                  >
                    Delete
                  </button>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {!campaigns.length && (
        <EmptyState compact icon={<IconSend size={20} />} title="No campaigns yet">
          Build one with Guided, Composer, or Express above.
        </EmptyState>
      )}
    </section>
  );
}

export function ResultsView({
  append,
  initialCampaignId,
  assessmentId = null,
  onOpenSession = null,
}) {
  const [campaigns, setCampaigns] = useState([]);
  const [id, setId] = useState(initialCampaignId || "");
  const [funnel, setFunnel] = useState(null);
  const [eventsText, setEventsText] = useState("");
  const [eventsOpen, setEventsOpen] = useState(false);
  const [importing, setImporting] = useState(false);

  useEffect(() => {
    invoke("list_campaigns", { assessmentId: assessmentId || null })
      .then((c) => {
        setCampaigns(c);
        if (!id && c[0]) setId(String(c[0].id));
      })
      .catch((e) => append(String(e)));
  }, [assessmentId]);

  useEffect(() => {
    if (initialCampaignId) setId(String(initialCampaignId));
  }, [initialCampaignId]);

  useEffect(() => {
    if (!id) return;
    const load = () =>
      invoke("campaign_funnel", { campaignId: id })
        .then(setFunnel)
      .catch((e) => append(String(e)));
    load();
    const t = setInterval(() => {
      invoke("campaign_funnel", { campaignId: id })
        .then(setFunnel)
        .catch(() => {});
    }, 2500);
    return () => clearInterval(t);
  }, [id]);

  const attempts = funnel?.attempts || [];
  const sent = funnel?.sent ?? 0;
  const lureHits = funnel?.lureHits ?? funnel?.lure_hits ?? 0;
  const captures = funnel?.captures ?? 0;
  const delivered = funnel?.delivered ?? 0;
  const opened = funnel?.opened ?? 0;
  const clicked = funnel?.clicked ?? 0;
  const bounced = funnel?.bounced ?? 0;
  const complained = funnel?.complained ?? 0;

  const importEvents = async () => {
    if (!id || !eventsText.trim()) return;
    setImporting(true);
    try {
      const r = await invoke("import_delivery_events", { campaignId: id, raw: eventsText });
      append(
        `Delivery events: parsed ${r.parsed}, matched ${r.matched}, updated ${r.updated}, unmatched ${r.unmatched}`
      );
      setEventsText("");
      setFunnel(await invoke("campaign_funnel", { campaignId: id }));
    } catch (e) {
      append(String(e));
    } finally {
      setImporting(false);
    }
  };

  const exportReport = async (format) => {
    if (!id) return;
    try {
      const text = await invoke("export_campaign_report", { campaignId: id, format });
      const cname =
        campaigns.find((c) => String(c.id) === String(id))?.name || "campaign";
      const path = await downloadText(
        `${cname}-report.${format === "csv" ? "csv" : "json"}`,
        text
      );
      if (!path) append("Export cancelled");
      else append(`Exported ${format.toUpperCase()} report for ${cname} → ${path}`);
    } catch (e) {
      append(String(e));
    }
  };

  return (
    <section className="card" data-testid="results-view">
      <div className="section-head">
        <h2 className="section-head-title">
          Results
          <Hint hint="Funnel: queued → sent/accepted (SMTP/API) → delivered → opened → clicked → lure visit → credential/session capture. Delivered / opened / clicked come from imported provider events." />
        </h2>
      </div>
      {!campaigns.length ? (
        <EmptyState compact icon={<IconChart size={20} />} title="No campaigns to report on">
          Launch a campaign to see its delivery and capture funnel here.
        </EmptyState>
      ) : null}
      <label className="block">
        Campaign
        <select
          data-testid="results-campaign"
          value={id || ""}
          onChange={(e) => setId(e.target.value)}
        >
          {campaigns.map((c) => (
            <option key={c.id} value={c.id}>
              {c.name} · {c.status} · {c.sent}/{c.sent + c.failed + c.pending}
            </option>
          ))}
        </select>
      </label>
      <div className="funnel-stats">
        <div>
          <span className="num">{sent}</span>
          <span className="lbl">Sent / accepted</span>
        </div>
        <div>
          <span className="num">{delivered}</span>
          <span className="lbl">Delivered</span>
        </div>
        <div>
          <span className="num">{opened}</span>
          <span className="lbl">Opened</span>
        </div>
        <div>
          <span className="num">{clicked}</span>
          <span className="lbl">Clicked</span>
        </div>
        <div>
          <span className="num">{bounced}</span>
          <span className="lbl">Bounced</span>
        </div>
      </div>
      <div className="funnel-stats">
        <div>
          <span className="num">{complained}</span>
          <span className="lbl">Complained</span>
        </div>
        <div>
          <span className="num">{lureHits}</span>
          <span className="lbl">Lure visits</span>
        </div>
        <div>
          <span className="num">{captures}</span>
          <span className="lbl">Captures</span>
        </div>
      </div>

      <div className="row results-toolbar">
        <button
          type="button"
          className="ghost"
          data-testid="results-export-csv"
          disabled={!id}
          onClick={() => exportReport("csv")}
        >
          Export CSV
        </button>
        <button
          type="button"
          className="ghost"
          data-testid="results-export-json"
          disabled={!id}
          onClick={() => exportReport("json")}
        >
          Export JSON
        </button>
        <button
          type="button"
          className="linkish"
          data-testid="results-import-toggle"
          onClick={() => setEventsOpen((v) => !v)}
        >
          {eventsOpen ? "▾" : "▸"} Import delivery events
        </button>
      </div>
      {eventsOpen && (
        <div className="events-import">
          <p className="muted small">
            Paste provider webhook JSON or an exported events report (Resend, SendGrid, Mailgun,
            Postmark, SES/SNS). Events are matched to recipients by message id, then email, to
            populate delivered / opened / clicked / bounced / complained.
          </p>
          <textarea
            data-testid="results-events"
            className="html-area"
            rows={5}
            value={eventsText}
            onChange={(e) => setEventsText(e.target.value)}
            placeholder='[{"email":"alice@client.com","event":"delivered"},{"email":"alice@client.com","event":"opened"}]'
          />
          <div className="row">
            <button
              data-testid="results-import"
              disabled={importing || !id || !eventsText.trim()}
              onClick={importEvents}
            >
              {importing ? "Importing…" : "Import events"}
            </button>
          </div>
        </div>
      )}
      <table>
        <thead>
          <tr>
            <th>Email</th>
            <th>Sent</th>
            <th>Deliv.</th>
            <th>Open</th>
            <th>Click</th>
            <th>Bounce</th>
            <th>Captured</th>
            <th>Session</th>
            <th>Error</th>
            <th>Sent at</th>
          </tr>
        </thead>
        <tbody>
          {attempts.map((a) => (
            <tr key={a.id}>
              <td className="mono">{a.email}</td>
              <td>{a.status === "sent" ? "accepted" : a.status}</td>
              <td>{a.delivered ? "✓" : "—"}</td>
              <td>{a.opened ? "✓" : "—"}</td>
              <td>{a.clicked ? "✓" : "—"}</td>
              <td title={a.bounceReason || a.bounce_reason || ""}>{a.bounced ? "✕" : "—"}</td>
              <td>{a.captured ? "yes" : "—"}</td>
              <td className="mono small">
                {(() => {
                  const raw = a.captureSessionId ?? a.capture_session_id;
                  if (raw == null || raw === "") return "—";
                  const sid = String(raw);
                  if (!onOpenSession) return sid;
                  return (
                    <button
                      type="button"
                      className="linkish"
                      onClick={() =>
                        onOpenSession({
                          sessionId: sid,
                          profileId: a.profileId ?? a.profile_id,
                          campaignId: id,
                        })
                      }
                    >
                      {sid.length > 12 ? `${sid.slice(0, 12)}…` : sid}
                    </button>
                  );
                })()}
              </td>
              <td className="small">{a.error || "—"}</td>
              <td className="small mono">{a.sentAt || a.sent_at || "—"}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {id && !attempts.length && (
        <p className="muted">No attempts for this campaign yet.</p>
      )}
    </section>
  );
}
