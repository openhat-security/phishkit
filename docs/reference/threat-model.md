# Threat model

phishkit is offensive-security tooling for **authorized** assessments. This page
states what it does, what it does not do, what it puts at risk, and where its
guardrails end. Read it before relying on phishkit for an engagement.

## What phishkit is

- A local desktop app that drives an AiTM proxy (evilginx) and a native email
  engine to run an authorized phishing assessment end to end.
- A tool that produces **real, sensitive artifacts**: captured credentials, live
  session cookies, and body tokens for accounts the target's users log into.

## What phishkit is not

- It is **not** a consent or legality oracle. The in-app acknowledgment records
  operator intent; it does not verify that a test is authorized or in scope.
- It is **not** a hosted service. It does not send mail for you, does not store
  your data off-machine, and does not phone home.
- It is **not** a generic one-click phishing service. Each engagement requires
  its own phishlet, content, and recipient list.
- It is **not** a guarantee of deliverability or of evading defenses; that
  depends on your domains, sending reputation, and the target's controls.

## Assets to protect

1. **Captured credentials and session tokens** — as sensitive as the accounts
   they unlock.
2. **Recipient PII** — the people in a recipient list are real.
3. **Sender credentials** — your SMTP/ESP API keys.
4. **Engagement scope and authorization records.**

## Actors

- **Operator (trusted, authorized).** The intended user, running phishkit under
  written authorization. phishkit optimizes for this actor's control and safety.
- **Assessed users (subjects).** People who receive lures. phishkit captures
  what they submit through the proxied flow.
- **Misuser (out of scope, adversarial).** Someone attempting unauthorized
  phishing. Enabling this actor is explicitly a non-goal; requests to do so are
  rejected.
- **Local attacker (post-capture).** Anyone who can read the operator machine
  after captures exist. This is why local files are the primary sensitive store.

## Guardrails

- **Authorized-use gate** before bulk send.
- **Awareness mode** for click-only training that never captures credentials.
- **Gated, allow-listed replay** for operator verification only.
- **Redaction by default** on exports and reports.
- **Snapshots** freeze what was sent, so audit history is not rewritten.
- **Assessment lifecycle** provides export, selective purge, and `/etc/hosts`
  cleanup so data can be destroyed when the engagement ends.
- **Local-only, no telemetry.** See [privacy](/reference/privacy).

## Limitations and residual risk

- The authorized-use acknowledgment is **operator attestation**, not proof of
  authorization; phishkit cannot stop a determined misuser who lies to it.
- Captured data is only as safe as the operator machine and the operator's
  handling after export. phishkit cannot protect exported files.
- Deliverability and detection depend on your infrastructure and the target's
  controls; ESP acceptable-use compliance is your responsibility.
- Elevated `/etc/hosts` edits modify the operator's own machine and require
  local admin rights; cleanup is provided but must be run.
- Only one campaign and one evilginx runtime run at a time in this phase; these
  constraints are surfaced in runtime state rather than as raw errors.
- phishkit is pre-1.0 alpha software with no audited stable release yet.

## Reporting weaknesses

Weaknesses in the authorized-use gate, allow-listed replay, or data-handling
guardrails are in scope and welcome. Report them privately per
[SECURITY.md](https://github.com/openhat-security/phishkit/blob/main/SECURITY.md),
not in a public issue.
