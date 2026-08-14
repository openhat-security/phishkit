# Authorized use

phishkit is built for security assessments conducted with **explicit written
authorization** from the owner of the targeted systems and people. Using it
without authorization may be a crime. If you cannot produce written
authorization for a specific target, do not use phishkit against it.

This project is dual-use security tooling. It is intended for ethical,
authorized testing — not as crimeware. Maintainers will reject issues, PRs, and
requests that seek to enable unauthorized phishing or to remove safety gates.

## What "authorized" means here

- **Written scope.** Email is acceptable, but you must have a record that names
  the systems, domains, and population you may test, from someone empowered to
  authorize it.
- **Per-engagement customization.** phishkit is deliberately not a one-click
  generic phishing service. Each engagement requires its own phishlet, content,
  and recipient list.
- **Real people are involved.** Recipients are humans. Capture and store only
  what your rules of engagement allow, and handle it as described in
  [privacy](/reference/privacy) and
  [local data and network activity](/reference/data-and-network).

## Prohibited use

Do **not** use phishkit to:

- Phish, proxy, or capture credentials for systems or people **without** written
  authorization from an empowered owner
- Target production consumer brands or third-party tenants using community
  phishlets as a “menu” of attacks outside a scoped engagement
- Evade lawful detection, abuse email providers, or violate ESP/SMTP acceptable
  use policies
- Request or contribute features whose primary purpose is unauthorized use

Local demos under `demos/` and vendored community YAML are for **lab learning
and authorized engagement preparation** only.

## Guardrails in the app

- A one-time **authorized-use acknowledgment** gates bulk send. It records the
  operator's assertion of authorization; it is not consent from the target and
  does not verify scope.
- **Session replay is gated and allow-listed.** It is for operator verification
  during an authorized engagement, not standing account takeover.
- **Awareness mode** runs click-only training campaigns that never capture
  credentials. Use it when the goal is metrics and education rather than proof
  of session compromise. See [campaigns](/guide/campaigns#awareness-mode).
- **Export and reporting redact by default** — recipient PII, credentials, and
  tokens are masked unless you explicitly request a full export.
- **Bring your own delivery.** phishkit sends through your SMTP relay or ESP API
  keys. You are responsible for provider acceptable-use compliance and for the
  domains you send from.
- CLI wizards (`phishkit wiz …`) require an affirmative authorized-use
  confirmation before proxy/mail steps.

## Your responsibilities

- Keep the written authorization on file for the duration of the engagement.
- Follow your firm's and the client's data-retention and destruction policy.
- Purge captured data when the engagement ends — the assessment lifecycle
  provides [selective purge and export](/guide/campaigns#assessment-lifecycle).
- Do not commit captured credentials, tokens, cookies, recipient PII, or `run/`
  state to source control.

Software cannot make abuse impossible. Intended use, prohibited use, and
operator responsibility are the project’s stance. See
[SECURITY.md](https://github.com/openhat-security/phishkit/blob/main/SECURITY.md).
