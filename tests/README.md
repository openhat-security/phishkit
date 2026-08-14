# Tests

phishkit tests live here, not under `docs/`.

| Directory | What it is | Command |
|-----------|------------|---------|
| [`unit/`](unit/) | Rust public-API tests (temp dirs, no host app-data) | `make test` / `make test-unit` |
| [`integration/`](integration/) | Desktop UI suite (WebdriverIO + real Tauri binary) | `make test-integration-docker` |

Pure helper tests also live next to the code as `#[cfg(test)]` modules in
`crates/phishkit-core/src/`. Those run with the same `make test-unit` command.

## What you should know

- **Never point tests at your real operator database.** Unit tests set
  `PHISHKIT_CONFIG` / `PHISHKIT_DATA` to a temp directory. The integration suite
  does the same (`/tmp/phishkit-test` in Docker, a fresh temp dir on the host).
- **`make test-integration-docker` is the default UI run.** It builds a Linux
  desktop binary and drives it on Xvfb inside Ubuntu. Your Mac desktop does not
  show a window and does not write to `~/Library/Application Support`.
- **`make test-integration` is debug-only.** It still sandboxes data, but it
  opens a real window on the host.
- **Video is opt-in.** `VIDEO=1 make test-integration` (or the docs remux
  target) records MP4s under `tests/integration/artifacts/`. Default runs stay
  assertion-first and faster.
- **Mailbox Destinations is not in the default suite.** That needs real
  credentials: `TEST_EMAIL` / `TEST_PASSWORD` and `make test-destinations`.
- **Linux Docker does not prove the macOS WKWebView path.** Operators on macOS
  should still smoke the app locally. CI continues to run `make test-unit` on
  macOS and Linux.

Full operator-facing write-up: [docs/guide/testing.md](../docs/guide/testing.md).
