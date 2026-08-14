# Security policy

phishkit drives an adversary-in-the-middle (AiTM) proxy, sends email, and
handles captured credentials and live session tokens. It also performs elevated
local operations (for example, editing `/etc/hosts`). Treat every security
report as sensitive.

## Supported versions

Only the latest tagged release receives security fixes. Until the project
publishes its first audited stable release, all builds are pre-1.0 alpha
software and must not be relied on for high-risk activity.

## Reporting a vulnerability

Do not open a public issue for vulnerabilities. Use the repository's private
[GitHub Security Advisory](https://github.com/openhat-security/phishkit/security/advisories/new)
form and include:

- the affected version and operating system;
- reproduction steps and expected impact;
- relevant logs with recipient emails, credentials, tokens, cookies, domains,
  and file paths removed;
- whether coordinated disclosure requires an embargo.

We aim to acknowledge reports within 72 hours, provide an initial assessment
within 7 days, and coordinate publication after a fix is available.

**Never** attach captured credentials, session cookies, JWTs, or unredacted
recipient lists to a report. If a report concerns leaked capture data, describe
the shape of the leak rather than pasting the data.

## Authorized use

phishkit is built for security assessments conducted with **explicit written
authorization** from the owner of the targeted systems and people. Using it
without authorization may be illegal. See
[docs/guide/authorized-use.md](docs/guide/authorized-use.md) (including
**Prohibited use**). A report that amounts to "help me phish someone without
permission" is out of scope and will be closed. Reports about weaknesses in the
authorized-use gate, allow-listed replay, or data-handling guardrails are in
scope and welcome.

## Security boundaries

- phishkit does not decide whether a test is authorized. The in-app
  acknowledgment records operator intent; it is not consent from the target.
- The AiTM proxy (evilginx) captures whatever the real login flow exposes to a
  proxied victim — credentials, cookies, and body tokens. Captured material is
  as sensitive as the accounts it unlocks.
- Delivery uses **your** SMTP relay or ESP API keys. Content, deliverability,
  and provider acceptable-use compliance are the operator's responsibility.
- Session replay is gated and allow-listed; it is intended for operator
  verification during an authorized engagement, not for standing account
  takeover.
- Local state (databases, captures, exports, logs) lives in the OS application
  data directory (or an ephemeral sandbox). phishkit does not phone home, but it
  cannot protect data after you export it.
- Elevated operations (`/etc/hosts` edits and their paired cleanup) change the
  operator's own machine and require local administrator rights.
- Awareness mode is click-only and does not capture credentials; do not treat a
  credential-capturing (AiTM) campaign as awareness training.

See the [threat model](docs/reference/threat-model.md) for the detailed model,
the [platform matrix](docs/reference/platform-support.md), and the
[local data and network inventory](docs/reference/data-and-network.md).
