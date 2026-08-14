# Desktop integration tests

WebdriverIO drives the **real** phishkit desktop binary (embedded WebDriver)
through every core console flow. Recordings are optional.

## Run (preferred)

From the kit root, with Docker running:

```bash
make test-integration-docker
```

This builds a Linux `phishkit` binary with `--features test-hooks,custom-protocol`,
starts the cookie-session demo on `:9080`, and runs the suite on Xvfb. Nothing
appears on your host desktop. Data stays in the container
(`PHISHKIT_CONFIG` / `PHISHKIT_DATA` under `/tmp/phishkit-test`).

## Host debug (opens a window)

```bash
make test-integration
```

Still uses a temp `PHISHKIT_*` sandbox. Do not omit those env vars or the app
will write to your real OS app-data directory.

## Video

```bash
VIDEO=1 make test-integration
# or remux into docs/media/:
make update-video-documentation-desktop
```

Artifacts: `tests/integration/artifacts/` (gitignored).

## Specs

| Spec | Flow |
|------|------|
| `01-setup.spec.ts` | First-run wizard → console |
| `02-assessment.spec.ts` | Create, export, clone, archive, restore |
| `03-mail.spec.ts` | Template, recipients, delivery presets (no SMTP send) |
| `04-campaign.spec.ts` | Guided / Composer / Express + Results |
| `05-target.spec.ts` | Cookie demo detect + Recon & Proxy chrome |
| `06-sessions.spec.ts` | Sync, filters, search |
| `07-settings.spec.ts` | Settings view |

Not in this suite: real SMTP send-test, Destinations mailbox login
(`make test-destinations`), host `/etc/hosts` admin prompts, community GitHub
sync.

## Build notes

Build with `custom-protocol` so `frontendDist` is embedded. Without it, Tauri
keeps `cfg(dev)` and loads `devUrl` (`http://localhost:1422`) — WebDriver sees
`about:blank` if Vite is not running.

Production builds omit plugin registration (`#[cfg(feature = "test-hooks")]`).
