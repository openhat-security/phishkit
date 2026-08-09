// Shared helpers for reasoning about evilginx capture rows.

export function isEmptyCapture(c) {
  const d = c?.data || {};
  const user = (d.username || "").trim();
  const pass = (d.password || "").trim();
  const custom = d.custom && typeof d.custom === "object" ? d.custom : {};
  const body = d.body_tokens && typeof d.body_tokens === "object" ? d.body_tokens : {};
  const cookies = d.tokens && typeof d.tokens === "object" ? d.tokens : {};
  return (
    !user &&
    !pass &&
    Object.keys(custom).length === 0 &&
    Object.keys(body).length === 0 &&
    Object.keys(cookies).length === 0
  );
}

export function formatAccessTime(c) {
  const d = c?.data || {};
  const epoch =
    c.evilginx_create_time ??
    c.evilginxCreateTime ??
    d.create_time ??
    c.evilginx_update_time ??
    c.evilginxUpdateTime ??
    d.update_time;
  if (!epoch) return "—";
  const ms = epoch < 1e12 ? epoch * 1000 : epoch;
  try {
    return new Date(ms).toLocaleString();
  } catch {
    return "—";
  }
}

export function captureAccessEpochMs(c) {
  const d = c?.data || {};
  const epoch =
    c.evilginx_update_time ??
    c.evilginxUpdateTime ??
    d.update_time ??
    c.evilginx_create_time ??
    c.evilginxCreateTime ??
    d.create_time;
  if (!epoch) return 0;
  return epoch < 1e12 ? epoch * 1000 : epoch;
}

export function captureHasSessionTokens(d) {
  const custom = d?.custom || {};
  const body = d?.body_tokens || {};
  return !!(
    custom.id_token ||
    custom.refresh_token ||
    body.access_token ||
    body.id_token ||
    d?.id_token ||
    d?.refresh_token
  );
}

export function cookieDomainCount(d) {
  const tokens = d?.tokens;
  if (!tokens || typeof tokens !== "object") return 0;
  return Object.keys(tokens).length;
}

export function sessionTimeline(c, mailHit) {
  const d = c?.data || {};
  const events = [];
  const created = c.evilginx_create_time ?? c.evilginxCreateTime ?? d.create_time;
  if (d.landing_url || created) {
    events.push({
      t: created,
      label: "Lure hit",
      detail: d.landing_url || "session created",
    });
  }
  if ((d.username || "").trim() || (d.password || "").trim()) {
    events.push({
      t: c.evilginx_update_time ?? c.evilginxUpdateTime ?? d.update_time ?? created,
      label: "Credentials",
      detail: d.username || "(captured)",
    });
  }
  const cookieN = cookieDomainCount(d);
  if (cookieN > 0 || captureHasSessionTokens(d)) {
    events.push({
      t: c.evilginx_update_time ?? c.evilginxUpdateTime ?? d.update_time ?? created,
      label: "Session tokens",
      detail: cookieN ? `${cookieN} cookie domain(s)` : "id/refresh/access tokens",
    });
  }
  if (mailHit) {
    events.push({
      t: null,
      label: "Matched mail send",
      detail: `${mailHit.campaignName || mailHit.campaign_name} · ${
        mailHit.sentAt || mailHit.sent_at || ""
      }`,
    });
  }
  return events;
}

/// Produce a redacted copy of a capture: usernames kept, secrets masked, token
/// and cookie *names* preserved but values replaced. For exportable evidence.
export function redactCapture(c) {
  const d = c?.data || {};
  const maskObjValues = (obj) => {
    const out = {};
    for (const k of Object.keys(obj || {})) out[k] = "REDACTED";
    return out;
  };
  const maskCookies = (tokens) => {
    const out = {};
    for (const domain of Object.keys(tokens || {})) {
      const names = tokens[domain] && typeof tokens[domain] === "object" ? tokens[domain] : {};
      out[domain] = Object.keys(names);
    }
    return out;
  };
  return {
    evilginx_session_id: c.evilginx_session_id ?? c.evilginxSessionId,
    landing_url: d.landing_url || "",
    username: d.username || "",
    password: d.password ? "REDACTED" : "",
    remote_addr: d.remote_addr || "",
    useragent: d.useragent || d.user_agent || "",
    phishlet: d.phishlet || "",
    create_time: c.evilginx_create_time ?? c.evilginxCreateTime ?? d.create_time ?? null,
    update_time: c.evilginx_update_time ?? c.evilginxUpdateTime ?? d.update_time ?? null,
    tokens: maskObjValues(d.custom),
    body_tokens: maskObjValues(d.body_tokens),
    cookie_domains: maskCookies(d.tokens),
  };
}
