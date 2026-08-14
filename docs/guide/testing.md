# Testing

How phishkit is tested, and what that means if you run the suite on a machine
you also use for assessments.

::: warning Authorized use only
Tests drive the real desktop app and the localhost demos. Never point them at
systems or mailboxes you are not authorized to assess. See
[authorized use](/guide/authorized-use).
:::

## Commands

| Command | What it does | Touches your Mac UI / app-data? |
|---------|--------------|----------------------------------|
| `make test` / `make test-unit` | `cargo fmt --check` + Rust tests (core + CLI) | No |
| `make lint` | Clippy on workspace packages | No |
| `make test-integration-docker` | Desktop UI suite in Ubuntu + Xvfb | **No** (preferred) |
| `make test-integration` | Same suite on the host | Window yes; app-data **no** (temp sandbox) |
| `VIDEO=1 make test-integration` | Same, plus MP4 artifacts | Window yes |
| `make test-destinations` | Destinations lure + Playwright | Needs `TEST_EMAIL` / `TEST_PASSWORD` |
| `make update-video-documentation` | Remux recordings into `docs/media/` | See [walkthrough videos](/guide/walkthrough) |

`make check` is an alias of `make test-unit` (CI still calls `make check`).

## Isolation (why Docker)

The desktop app normally stores SQLite and `setup.json` in the OS app-data /
config directories (`~/Library/Application Support/com.phishkit.phishkit` on
macOS). A UI test that launches the real binary **will use those paths unless
you override them**.

phishkit honors:

- `PHISHKIT_CONFIG` — `setup.json` and durable preferences
- `PHISHKIT_DATA` — database and evilginx runtime
- `PHISHKIT_ROOT` — kit sources (`kit/evilginx`, `vendor/`)
- `PHISHKIT_TEST_BIN` — path to the desktop binary for WebdriverIO

`make test-integration-docker` sets the first two to `/tmp/phishkit-test` inside
the container. `make test-integration` creates a fresh temp directory on the
host. **Do not run the WebdriverIO suite against your live assessment database.**

Docker cannot run the macOS `.app`. The container builds the **Linux** Tauri
binary and draws it on a virtual framebuffer (Xvfb). Recordings show GTK /
WebKit chrome, not native macOS window chrome. That is the tradeoff for not
stealing focus on your desktop.

## What the UI suite covers

Every core sidebar flow in a throwaway sandbox: Setup, Assessments (including
clone / archive / restore), Templates, Recipients, Delivery presets (no send),
Campaigns (Guided / Composer / Express), Results, Targets + cookie-demo detect,
Recon & Proxy chrome, Sessions filters, Settings.

Intentionally **not** in the default Docker job:

- Real SMTP / ESP send-test (needs a mailbox)
- Destinations lure login (`make test-destinations`)
- Writing the host `/etc/hosts` file
- Community phishlet sync against GitHub

## Layout

```
tests/
  unit/            # Rust public-API tests (Cargo)
  integration/     # WebdriverIO + Docker
```

Helper unit tests also live as `#[cfg(test)]` next to the functions in
`crates/phishkit-core/src/`.

See [`tests/README.md`](https://github.com/irruptio-security/phishkit/blob/main/tests/README.md)
in the repository.
