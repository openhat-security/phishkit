# What phishkit is

::: warning Alpha
This is a **primitive alpha** (`v0.1.0`), not a beta and not production. We are
still putting the product together. Build from source, expect breakage, and
read [authorized use](/guide/authorized-use) before you run anything.
:::

phishkit is a desktop application for running **authorized** phishing
assessments end to end. It wraps [evilginx](https://github.com/kgretzky/evilginx2)
(an adversary-in-the-middle, or AiTM, proxy) together with a native email
campaign engine so a single operator can take an engagement from setup to
captured session without stitching separate tools together.

The supported product is the local Tauri/React/Rust app under `apps/desktop/`. It
runs on your machine; it does not host mail for you and does not phone home.

## The workflow

```
Assessment → Target → Phishlet / Proxy → Named Lure → Template →
Recipients → Campaign → Results → Session
```

- **Assessment** — the top-level container for an engagement. Targets, lures,
  templates, recipient lists, campaigns, and captured sessions all belong to it.
- **Target** — a domain/profile you are authorized to assess. Each target gets
  a phishlet and an evilginx destination.
- **Phishlet / Proxy** — evilginx proxies the real login flow and captures
  credentials plus session tokens (cookies, body JWTs).
- **Named Lure** — a tracked link into the proxy. Attempts carry a token so
  captures can be attributed deterministically.
- **Template + Recipients** — HTML email with merge tags and an imported
  recipient list, scoped to the assessment.
- **Campaign** — a draft you review, test, and launch. Sender identity and
  rendered content are snapshotted at creation for auditability.
- **Results** — the funnel: queued, sent/accepted, delivered, opened, clicked,
  bounced, complained, lure visits, and captures.
- **Session** — a captured session with a timeline, masked credentials, a
  token/cookie summary, campaign/lure attribution, export, and gated replay.

## Two audiences, one tool

- **Technical operators** get full Advanced control: custom phishlets, sender
  accounts (SMTP/SES/Resend/SendGrid/Mailgun/Postmark), scheduling and send
  windows, rate limits, delivery-event ingestion, and a headless
  [CLI](/guide/cli).
- **Business users** get a [guided campaign wizard](/guide/campaigns#guided) and
  a curated preset scenario library with safe defaults and inline "why" — enough
  to run a safe click-through awareness campaign out of the box.

## Next steps

- [Authorized use](/guide/authorized-use) — required reading before you run anything.
- [Install](/guide/install) — prerequisites and build.
- [Quick start](/guide/quick-start) — a local, self-contained dry-run.
- [Campaign guide](/guide/campaigns) — compose, review, test, and launch.
