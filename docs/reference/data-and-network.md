# Data and network

Operator state defaults to **OS application directories** (not the git checkout):

| Kind | Location |
|------|----------|
| Config (`setup.json`) | macOS Application Support / XDG config / `%APPDATA%` under `phishkit` |
| App database | OS data dir `phishkit.db` (or custom path / ephemeral temp sandbox from Setup) |
| evilginx runtime | OS data dir `evilginx/` (`config.json`, buntdb, logs) |
| Kit sources | Clone or install tree: `kit/evilginx/`, `vendor/` |
| Legacy | One-time migrate from checkout `run/**/phishkit.db` |

Override with `PHISHKIT_CONFIG`, `PHISHKIT_DATA`, and (for scripts) `EVILGINX_DATA_DIR`.
Inspect resolved paths with `phishkit paths` or Settings in the desktop app.

## Local data

| Data | Location | Notes |
|------|----------|-------|
| Application database | OS data dir `phishkit.db` | Assessments, targets, lures, templates, recipient lists, campaigns, snapshots, attempts, synced captures. SQLite. |
| Legacy database migration | Checkout `run/**/phishkit.db` → OS data dir (`.bak` left behind) | One-time on first run; preserves existing user data. |
| evilginx captures | OS data dir `evilginx/` (legacy: `kit/evilginx/run/data/`) | Credentials and session tokens captured by the proxy. |
| evilginx binary | `kit/evilginx/run/evilginx` | Built locally; immutable kit asset. |
| Sender settings | Application database | Your SMTP/ESP configuration and keys. |
| Exports | Operator-chosen paths | Report CSV/JSON, cookies.txt/JSON, assessment bundles. |
| Community phishlet packs | `vendor/community-phishlets/` | Vendored in-repo; pinned in a lockfile; refresh with `make community-phishlets`. |

Mutable state under the OS data directory (and any leftover gitignored `run/` /
`kit/evilginx/run/` paths) is excluded from commits. `.db`, key material, and
`.env` secrets are gitignored.

## Network activity

| Activity | Destination | When | Data |
|----------|-------------|------|------|
| Email delivery | **Your** SMTP relay or ESP API (SES/Resend/SendGrid/Mailgun/Postmark) | On test send and campaign launch | Rendered message, recipient address, sender identity |
| Delivery events | Your ESP (API), or pasted/imported JSON | On event ingestion | Provider event records reconciled by message ID |
| AiTM proxy | The **real target** application, and the assessed user | While a lure is live | Proxied login traffic; captured credentials/tokens land locally |
| Target recon / detect | The target URL | During detect/scaffold | Login-flow fingerprinting requests |
| Community phishlet sync | GitHub (pinned commits) | Only when you run the sync | Downloads phishlet packs |

phishkit makes **no** analytics, telemetry, crash-reporting, or update-check
requests. It does not send captured data anywhere; captures are written locally
and only leave the machine if you export them.

## Elevated operations

| Operation | Platform | Purpose |
|-----------|----------|---------|
| `/etc/hosts` add | macOS (admin prompt) | Point the local dry-run domain at the proxy |
| `/etc/hosts` remove | macOS (admin prompt) | Paired cleanup at end of engagement |

These modify the operator's own machine only. See
[platform support](/reference/platform-support) for the current matrix.
