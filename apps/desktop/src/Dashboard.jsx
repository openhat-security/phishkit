import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  IconChart,
  IconMail,
  IconSend,
  IconSliders,
  IconTarget,
  IconUsers,
  IconZap,
} from "./lib/icons";

const EMPTY = {
  profiles: [],
  templates: [],
  lists: [],
  campaigns: [],
  accounts: [],
  aup: { accepted: false },
  captures: [],
};

function hasCreds(c) {
  return !!(c?.data?.username || "").trim();
}

export default function Dashboard({ kit, status, append, go, goResults, refreshChrome }) {
  const [data, setData] = useState(EMPTY);

  const load = useCallback(async () => {
    try {
      const [profiles, templates, lists, campaigns, accounts, aup] =
        await Promise.all([
          invoke("list_profiles"),
          invoke("list_email_templates"),
          invoke("list_recipient_lists"),
          invoke("list_campaigns"),
          invoke("list_mail_accounts").catch(() => []),
          invoke("get_aup_status").catch(() => ({ accepted: false })),
        ]);
      const capLists = await Promise.all(
        profiles.map((p) =>
          invoke("list_captures", { profileId: p.id }).catch(() => [])
        )
      );
      const captures = capLists.flat();
      setData({ profiles, templates, lists, campaigns, accounts, aup, captures });
    } catch (e) {
      append(String(e));
    }
  }, [append]);

  useEffect(() => {
    load();
    const t = setInterval(load, 6000);
    return () => clearInterval(t);
  }, [load]);

  const { profiles, templates, lists, campaigns, accounts, aup, captures } = data;

  const recipientsTotal = lists.reduce(
    (n, l) => n + (l.recipientCount ?? l.recipient_count ?? 0),
    0
  );
  const emailsSent = campaigns.reduce((n, c) => n + (c.sent || 0), 0);
  const capturesWithCreds = captures.filter(hasCreds).length;
  const proxyUp = !!status?.evilginx_running;

  const acceptAup = async () => {
    try {
      await invoke("accept_aup");
      append("Authorized-use acknowledgment recorded");
      load();
    } catch (e) {
      append(String(e));
    }
  };

  const checklist = [
    {
      id: "bin",
      done: !!kit?.evilginx_bin,
      title: "Build the AiTM engine",
      sub: kit?.evilginx_bin
        ? "evilginx binary is present"
        : "Destinations → Advanced → Build binaries",
      action: () => go("destinations"),
      cta: "Open",
    },
    {
      id: "dest",
      done: profiles.some((p) => p.phishlet),
      title: "Create a destination",
      sub: profiles.some((p) => p.phishlet)
        ? `${profiles.length} destination${profiles.length === 1 ? "" : "s"} configured`
        : "Generate or import a phishlet for your target",
      action: () => go("destinations"),
      cta: "Build",
    },
    {
      id: "mail",
      done: accounts.length > 0,
      title: "Connect a sender",
      sub: accounts.length
        ? `${accounts.length} sender${accounts.length === 1 ? "" : "s"} saved`
        : "SMTP or ESP API key for delivery",
      action: () => go("settings"),
      cta: "Configure",
    },
    {
      id: "rcpts",
      done: recipientsTotal > 0,
      title: "Import recipients",
      sub: recipientsTotal
        ? `${recipientsTotal} recipient${recipientsTotal === 1 ? "" : "s"} across ${lists.length} list${lists.length === 1 ? "" : "s"}`
        : "Paste or upload a target list (scope only)",
      action: () => go("recipients"),
      cta: "Add",
    },
    {
      id: "aup",
      done: !!aup.accepted,
      title: "Acknowledge authorized use",
      sub: aup.accepted
        ? "Acknowledgment on file"
        : "Required before any campaign send",
      action: acceptAup,
      cta: "Acknowledge",
    },
  ];
  const remaining = checklist.filter((c) => !c.done).length;

  const stats = [
    { lbl: "Destinations", num: profiles.length, Icon: IconTarget },
    {
      lbl: "Sessions captured",
      num: captures.length,
      accent: true,
      Icon: IconZap,
      hint: capturesWithCreds
        ? `${capturesWithCreds} with credentials`
        : "credentials + tokens appear here",
    },
    { lbl: "Emails sent", num: emailsSent, Icon: IconSend },
    { lbl: "Campaigns", num: campaigns.length, Icon: IconChart },
    { lbl: "Recipients", num: recipientsTotal, Icon: IconUsers },
    { lbl: "Templates", num: templates.length, Icon: IconMail },
  ];

  const recent = campaigns.slice(0, 6);

  const quick = [
    {
      Icon: IconTarget,
      title: "New destination",
      sub: "Recon a target and mint an AiTM lure",
      to: "destinations",
    },
    {
      Icon: IconMail,
      title: "Design a template",
      sub: "Import HTML and drop in {{link}}",
      to: "templates",
    },
    {
      Icon: IconSend,
      title: "Launch a campaign",
      sub: "Bind a tracked link and send",
      to: "campaigns",
    },
    {
      Icon: IconSliders,
      title: "Configure delivery",
      sub: "Add an SMTP or ESP sender",
      to: "settings",
    },
  ];

  return (
    <>
      <section className="card">
        <div className="dash-hero">
          <div className="headline">
            <h2>Engagement overview</h2>
            <p className="muted">
              {profiles.length
                ? `${profiles.length} destination${profiles.length === 1 ? "" : "s"} · proxy ${proxyUp ? "live" : "idle"} · ${captures.length} session${captures.length === 1 ? "" : "s"} captured`
                : "Build your first AiTM destination to get started."}
            </p>
          </div>
          <button type="button" onClick={() => go("destinations")}>
            {profiles.length ? "Go to destinations" : "Get started"}
          </button>
        </div>
      </section>

      <div className="stat-grid">
        {stats.map((s) => (
          <div className="stat" key={s.lbl}>
            <span className="ico">
              <s.Icon size={20} />
            </span>
            <span className={`num ${s.accent ? "accent" : ""}`}>{s.num}</span>
            <span className="lbl">{s.lbl}</span>
            {s.hint && <span className="hint-line">{s.hint}</span>}
          </div>
        ))}
      </div>

      <div className="two-col">
        <section className="card">
          <h2>Recent campaigns</h2>
          {recent.length ? (
            <table>
              <thead>
                <tr>
                  <th>Name</th>
                  <th>Status</th>
                  <th>Progress</th>
                  <th>Sent</th>
                </tr>
              </thead>
              <tbody>
                {recent.map((c) => {
                  const pct = Math.round(c.progressPct ?? c.progress_pct ?? 0);
                  return (
                    <tr key={c.id}>
                      <td>
                        <button
                          type="button"
                          className="linkish"
                          onClick={() =>
                            goResults ? goResults(c.id) : go("results")
                          }
                        >
                          {c.name}
                        </button>
                      </td>
                      <td>
                        <span className={`pill-status ${c.status}`}>{c.status}</span>
                      </td>
                      <td>
                        <div className="progress-wrap" title={`${pct}%`}>
                          <div
                            className="progress-bar"
                            style={{ width: `${Math.min(100, pct)}%` }}
                          />
                        </div>
                        <span className="small muted">{pct}%</span>
                      </td>
                      <td className="mono">
                        {c.sent}/{c.total ?? c.sent + c.failed + c.pending}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          ) : (
            <p className="empty-hint">
              No campaigns yet.{" "}
              <button className="linkish" onClick={() => go("campaigns")}>
                Create one →
              </button>
            </p>
          )}
        </section>

        <section className="card">
          <h2>
            Setup {remaining ? `· ${remaining} left` : "· complete"}
          </h2>
          <ul className="checklist">
            {checklist.map((c) => (
              <li className="check-item" key={c.id}>
                <span className={`mark ${c.done ? "done" : "todo"}`}>
                  {c.done ? "✓" : ""}
                </span>
                <div className="ci-body">
                  <div className="ci-title">{c.title}</div>
                  <div className="ci-sub">{c.sub}</div>
                </div>
                {!c.done && (
                  <button className="ghost" onClick={c.action}>
                    {c.cta}
                  </button>
                )}
              </li>
            ))}
          </ul>
        </section>
      </div>

      <section className="card">
        <h2>Quick actions</h2>
        <div className="quick-actions">
          {quick.map((q) => (
            <button
              type="button"
              className="quick-card"
              key={q.to}
              onClick={() => go(q.to)}
            >
              <span className="qc-ico">
                <q.Icon size={18} />
              </span>
              <span className="qc-title">{q.title}</span>
              <span className="qc-sub">{q.sub}</span>
            </button>
          ))}
        </div>
      </section>
    </>
  );
}
