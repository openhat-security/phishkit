export const DELIVERY_HINTS = {
  transport: {
    title: "Delivery transport",
    body: `Gmail SMTP + App Password is the fastest path for small authorized sends.

For larger campaigns, prefer Amazon SES SMTP or self-hosted mail on a dedicated simulation domain.

HTTP adapters (Resend, SendGrid, …) are BYO API keys — check each ESP’s AUP for phishing-sim content.`,
  },
  gmail: {
    title: "Gmail SMTP (App Password)",
    body: `1. Google Account → Security → 2-Step Verification (required)
2. App passwords → Mail → generate 16-character password
3. Enter your Gmail address + that app password here (not your normal password)

Host/port are filled for you (smtp.gmail.com:587). Google may throttle or flag phishing-looking content — keep volume modest.`,
  },
  espAdapter: {
    title: "ESP / API providers",
    body: `Bring your own API key. You are responsible for the provider’s acceptable use policy.

For red-team control, SES SMTP or self-hosted mail on your domain is usually safer than a shared ESP pool.`,
  },
  smtpHost: {
    title: "SMTP host",
    body: `Use your SES regional endpoint or VPS mail hostname.

Gmail preset already sets smtp.gmail.com — you only need email + app password.`,
  },
  fromDomain: {
    title: "From address / domain",
    body: `For Gmail, From must be the same Gmail (or a Send-as alias you’ve verified).

For SES/self-hosted, use a dedicated simulation domain with SPF/DKIM/DMARC.`,
  },
  dnsAuth: {
    title: "DNS authentication",
    body: `1. SPF — authorize this SMTP host/IP
2. DKIM — sign with the sim domain
3. DMARC — start with p=none while warming

Gmail handles auth for @gmail.com sends; custom domains still need DNS.`,
  },
  rateLimit: {
    title: "Send rate",
    body: `Gmail: stay low (about 5–15/min). Google rate-limits and may lock the account on bursts.

Warm gradually on SES/self-hosted. Respect provider caps.`,
  },
  internalAwareness: {
    title: "Internal awareness campaigns",
    body: `Best inbox placement: send via the client’s approved relay and add a narrow allowlist for the sim domain or sending IP only — not a global spam bypass.`,
  },
  campaignSend: {
    title: "Before you send",
    body: `Authorized targets only (written scope).

Deliverability depends on your mailbox reputation — not this app.`,
  },
  linkUrl: {
    title: "Campaign link",
    body: `Use the tracked link from Destinations (AiTM lure), or paste any static URL.

Templates should include {{link}} so each recipient gets this destination.`,
  },
};

const CONSUMER_HOSTS = [
  "smtp.gmail.com",
  "smtp.google.com",
  "smtp-relay.gmail.com",
  "smtp.office365.com",
  "smtp-mail.outlook.com",
  "smtp.live.com",
];

export function isConsumerSmtpHost(host) {
  const h = (host || "").trim().toLowerCase();
  return CONSUMER_HOSTS.some((c) => h === c || h.endsWith(`.${c}`));
}

export function isGmailPreset(provider, host) {
  return (
    provider === "gmail" ||
    (host || "").toLowerCase().includes("smtp.gmail.com")
  );
}

export const GMAIL_NOTE =
  "Gmail SMTP uses an App Password (not your normal password). Good for small authorized batches — Google may throttle phishing-looking mail or larger volume.";

export const CONSUMER_SMTP_WARNING =
  "Microsoft/consumer SMTP: fine for a single test. Prefer Gmail App Password preset, SES, or self-hosted for real runs.";

export const STARTER_TEMPLATE = {
  name: "Lure link",
  subject: "Action required",
  html: `<p>Hi {{first_name}},</p>
<p>Please review this item at your earliest convenience:</p>
<p><a href="{{link}}">Continue</a></p>
<p>If the button does not work, open: {{link}}</p>`,
};
