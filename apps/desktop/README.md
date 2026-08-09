# phishkit desktop (Tauri + Rust)

Dual-core local control plane for **authorized** assessments:

1. **Destinations (AiTM)** — build/import proxy configs, start evilginx, mint tracked links, sync captures  
2. **Mail (BYO)** — HTML templates, recipient CSV, campaign send via **your** SMTP or ESP API keys  

No hosted mail. Credentials stay on the operator machine.

## Nav

| Tab | Role |
|-----|------|
| Dashboard | Engagement overview: stats, setup checklist, recent campaigns, quick actions |
| Destinations | Target → destination config → AiTM proxy → tracked link → captures |
| Templates | HTML + merge tags (`{{first_name}}`, `{{email}}`, `{{link}}`, …) |
| Recipients | Lists + CSV import |
| Campaigns | Bind AiTM link → rate-limited send |
| Results | Per-recipient send status |
| Delivery | Sender accounts: SMTP / SES SMTP / Resend / SendGrid / Mailgun / Postmark |

The desktop app is **evilginx-only**; kit discovery only requires the
`kit/evilginx/` tree, and service status reports the AiTM proxy.

## Flagship loop

1. Create a destination and start the proxy → copy **tracked link**  
2. Save an HTML template that includes `{{link}}`  
3. Import recipients (CSV with `email` column)  
4. Configure delivery in Settings (prefer SES SMTP or self-hosted on a **dedicated sim domain**)  
5. Create & start a campaign → watch progress / Results  

Hover the **?** hints on Settings/Campaigns for SPF/DKIM/DMARC and provider guidance. Gmail/O365 SMTP is labeled test-only.

## Delivery providers

| Provider | Mode | Notes |
|----------|------|--------|
| SMTP | Raw | Any relay (self-hosted, corporate) |
| Amazon SES | SMTP | Region → `email-smtp.{region}.amazonaws.com` |
| Resend / SendGrid / Mailgun / Postmark | HTTP API | BYO API key; check ESP AUP for phishing-sim content |

## Run

```bash
# from kit root
make desktop

# or
cd desktop && npm install && npm run tauri dev
```

Requires Rust (`~/.cargo/bin` on PATH), Node 18+, and once: `make build-evilginx`.

Set `PHISHKIT_ROOT` if kit discovery fails (dev defaults to repo root).

## Destinations e2e (steps 1–4)

Integration test that drives the same control-plane paths as the Destinations
UI (`phishkit_ctl`), then uses Playwright to open the lure and submit login:

```bash
E2E_EMAIL='you@example.com' E2E_PASSWORD='…' make e2e-destinations

# optional
E2E_TARGET=demo-cookie.local.phishkit E2E_HEADED=1 E2E_KEEP_PROXY=1 \
  E2E_EMAIL=… E2E_PASSWORD=… make e2e-destinations
```

Artifacts land in `run/e2e/` (`result.json`, screenshots). Passes when a
capture has username+password and/or Firebase tokens. Prefer the localhost
demos (`make demo-cookie` / `make demo-firebase`) for first-run practice —
see `demos/`.

## Data

- App database: owned by the Rust engine under the OS application data path
  (`phishkit paths` / Settings).
- Community packs: `vendor/community-phishlets/` (vendored in-repo; refresh pins
  with `make community-phishlets` / lockfile in `kit/evilginx/community-phishlets.lock.json`)
