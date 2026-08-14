# Quick start

This walks a **local, self-contained** dry-run against a target you are
authorized to assess. Everything stays on your machine.

::: warning Authorization first
Do not point phishkit at systems or people you are not authorized to test. Read
[authorized use](/guide/authorized-use).
:::

## 0. (Recommended) Run a localhost demo first

Practice capture shapes without a client target:

```bash
make demo-cookie          # cookie session on http://127.0.0.1:9080
# or: make demo-firebase  # Firebase-shaped mock on :9081
make validate-phishlet PHISHLET=demo-cookie
```

Test credentials: `demo@phishkit.local` / `demo-password`. Copy-ready Target
notes live under `demos/cookie/` and `demos/firebase/` in the
repo. See also `demos/README.md`.

Prefer the in-app **Demo tour** (sidebar) or the [walkthrough videos](/guide/walkthrough)
first — they cover
the desktop Assessment flow and both localhost demos.

## 1. Launch the app

```bash
make desktop
```

Accept the one-time authorized-use acknowledgment when prompted (required before
bulk send).

## 2. Create an assessment

From the Assessments home, create an assessment for the engagement. It becomes
the container for everything below.

## 3. Add a target and start the proxy

1. Add a **Target** for a domain you are authorized to assess — or use
   `demo-cookie.local.phishkit` / phishlet `demo-cookie` for local practice.
2. Generate or import its phishlet, then start the **evilginx** destination.
3. Apply the `/etc/hosts` entries when prompted (admin rights required for the
   local dry-run).
4. Copy the **tracked lure link**.

## 4. Add content

1. Save a **Template** — HTML with merge tags such as `{{first_name}}`,
   `{{email}}`, and `{{link}}`.
2. Create a **Recipient list** and import a CSV with an `email` column. The
   import preview validates and de-duplicates rows.

## 5. Configure a sender

In Delivery, add a sending account. For a local dry-run, use a test mailbox you
control. For real engagements prefer a dedicated simulation domain with correct
SPF/DKIM/DMARC. Supported: SMTP, Amazon SES (SMTP), and the Resend / SendGrid /
Mailgun / Postmark HTTP APIs.

## 6. Compose, review, test, launch

Open **Campaigns** and use the composer:

1. **Draft** — bind the target, named lure, sender, template, and list.
2. **Review** — phishkit runs readiness checks and surfaces missing
   dependencies as actionable cards.
3. **Test** — send a single message to yourself.
4. **Launch** — start sending (optionally scheduled, with a send window and
   rate limit).

Business users can instead pick the [Guided](/guide/campaigns#guided) flow and a
preset scenario.

## 7. Watch results and open the session

- **Results** shows the funnel: queued, sent, delivered, opened, clicked,
  bounced, complained, lure visits, and captures.
- Click a captured result to open the **Session** — timeline, masked
  credentials, token/cookie summary, and campaign/lure attribution — where you
  can export (cookies.txt / JSON / redacted bundle) or run gated replay.

## 8. Clean up

When the engagement ends, use the [assessment
lifecycle](/guide/campaigns#assessment-lifecycle) to export a bundle, purge
sessions/attempts/PII, and remove the `/etc/hosts` entries.

## Scripted end-to-end

The same paths are scriptable with the [CLI](/guide/cli). The repo ships a
Destinations end-to-end check:

```bash
TEST_EMAIL='you@example.com' TEST_PASSWORD='…' make test-destinations
```

The default desktop UI suite (no mailbox) is `make test-integration-docker`.
See [Testing](/guide/testing).
