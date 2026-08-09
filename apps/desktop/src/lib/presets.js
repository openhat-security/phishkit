// Curated scenario presets for the Guided campaign wizard. Each preset carries
// safe defaults (rate, mode), a ready email template (with {{first_name}} and
// {{link}} merge tags), a recommended phishlet pattern from
// evilginx/phishlet-templates/catalog.json, and inline "why" guidance so a
// non-technical operator can launch a defensible campaign out of the box.

function tmpl(name, subject, bodyLines) {
  return { name, subject, html: bodyLines.join("\n") };
}

export const SCENARIO_PRESETS = [
  {
    id: "m365-login",
    label: "Microsoft 365 sign-in",
    category: "Credential / AiTM",
    mode: "aitm",
    rate: 10,
    blurb:
      "Entra/Office 365 re-authentication prompt. Pair with an AiTM lure to capture the MFA-backed session, not just the password.",
    why: "AiTM (evilginx) proxies the real Microsoft login, so session cookies survive MFA — the realistic outcome an attacker gets.",
    phishletPattern: "oauth-oidc",
    recipientHint: "Staff mailboxes on the target tenant (written scope only).",
    lure: { ogTitle: "Microsoft 365", ogDesc: "Sign in to continue" },
    template: tmpl(
      "M365 — session re-auth",
      "[Action required] Reconfirm your Microsoft 365 session",
      [
        "<p>Hi {{first_name}},</p>",
        "<p>Your Microsoft 365 session for this device needs to be reconfirmed to keep email and Teams working.</p>",
        '<p><a href="{{link}}">Reconfirm your account</a></p>',
        "<p>If the button does not work, paste this link into your browser:<br>{{link}}</p>",
        "<p>Thank you,<br>IT Service Desk</p>",
      ]
    ),
  },
  {
    id: "okta-sso",
    label: "Okta SSO",
    category: "Credential / AiTM",
    mode: "aitm",
    rate: 10,
    blurb: "Okta/identity-provider SSO prompt for orgs that centralize logins behind Okta.",
    why: "OAuth/OIDC phishlets mirror the IdP host + cookie patterns so the captured session unlocks downstream SSO apps.",
    phishletPattern: "oauth-oidc",
    recipientHint: "Employees who authenticate through the Okta tenant.",
    lure: { ogTitle: "Okta", ogDesc: "Single sign-on" },
    template: tmpl(
      "Okta — verification required",
      "Verify your identity to keep SSO access",
      [
        "<p>Hi {{first_name}},</p>",
        "<p>We detected a new device on your single sign-on account. Verify it now to avoid a lockout.</p>",
        '<p><a href="{{link}}">Verify this device</a></p>',
        "<p>Link: {{link}}</p>",
        "<p>Security Operations</p>",
      ]
    ),
  },
  {
    id: "google-workspace",
    label: "Google Workspace",
    category: "Credential / AiTM",
    mode: "aitm",
    rate: 8,
    blurb: "Google Workspace security prompt. Keep the rate low — Google is aggressive about throttling.",
    why: "Cookie-session phishlets capture the Google auth cookies; the low default rate reduces sender-reputation damage.",
    phishletPattern: "cookie-sso",
    recipientHint: "Workspace users in the target domain.",
    lure: { ogTitle: "Google", ogDesc: "Security checkup" },
    template: tmpl(
      "Workspace — security checkup",
      "Complete your Workspace security checkup",
      [
        "<p>Hi {{first_name}},</p>",
        "<p>A quick security checkup is required to keep your Google Workspace apps active.</p>",
        '<p><a href="{{link}}">Start security checkup</a></p>',
        "<p>Or open: {{link}}</p>",
        "<p>Workspace Admin</p>",
      ]
    ),
  },
  {
    id: "generic",
    label: "Generic company login",
    category: "Credential / AiTM",
    mode: "aitm",
    rate: 12,
    blurb: "A neutral internal login for bespoke or in-house web apps.",
    why: "The Generic SPA phishlet handles app + API hosts and optional body/cookie tokens for custom stacks.",
    phishletPattern: "generic-spa",
    recipientHint: "Users of the internal application in scope.",
    lure: { ogTitle: "Company Portal", ogDesc: "Sign in" },
    template: tmpl(
      "Portal — document shared",
      "A document was shared with you",
      [
        "<p>Hi {{first_name}},</p>",
        "<p>A document was shared with you on the company site and is awaiting your review.</p>",
        '<p><a href="{{link}}">Open the document</a></p>',
        "<p>Direct link: {{link}}</p>",
      ]
    ),
  },
  {
    id: "awareness-payroll",
    label: "Awareness: payroll update (click-only)",
    category: "Awareness training",
    mode: "awareness",
    rate: 20,
    blurb:
      "Pure click-through awareness test. Never captures credentials — the link should point to a training or redirector page.",
    why: "Awareness mode measures who clicks without collecting any credentials, ideal for board-safe security training.",
    phishletPattern: null,
    recipientHint: "Any staff cohort you are authorized to train.",
    lure: { ogTitle: "Payroll", ogDesc: "Update your details" },
    template: tmpl(
      "Payroll — confirm your details",
      "Confirm your payroll details before Friday",
      [
        "<p>Hi {{first_name}},</p>",
        "<p>Payroll is updating direct-deposit records. Please confirm your details before Friday.</p>",
        '<p><a href="{{link}}">Confirm payroll details</a></p>',
        "<p>Link: {{link}}</p>",
        "<p>People Operations</p>",
      ]
    ),
  },
];

export function presetById(id) {
  return SCENARIO_PRESETS.find((p) => p.id === id) || null;
}
