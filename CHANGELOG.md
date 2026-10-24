# Changelog

All notable changes to phishkit are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1-alpha.1] - 2026-10-19

### Fixed

- AiTM cookie demo now runs end to end. The `demo-cookie` phishlet proxies to a
  reachable HTTPS upstream (`app.cookie-demo.test`) that is distinct from the lure
  host, so evilginx no longer dials its own hostname and loops.
- The lure launcher's bind poll no longer reports a false negative. It probes the
  configured HTTPS port with a socket connect (with `ss`/`lsof` as a fallback),
  re-checks once after the process is confirmed up, and binds the port read from
  the config instead of a hard-coded `443`.
- A failed lure start tears down its `screen` session and any half-bound port
  instead of leaving them running.
- `ensure-destination` reuses an existing phishlet that already serves the same
  landing host rather than scaffolding a duplicate that collides at load time.
- `upstream_domain` handles single-label internal hosts and compound public
  suffixes (`co.uk`, `github.io`, and similar) instead of always stripping the
  left label.

- SMTP passwords and API keys are sealed at rest (ChaCha20-Poly1305 under a
  local `master.key`), not left as plaintext in SQLite.
- The SQLite store and its data directory are forced to `0600` / `0700` on every
  open, including after a loose restore.
- Recon/`detect` verifies TLS by default and refuses loopback, RFC1918,
  link-local, CGNAT, and ULA targets unless `PHISHKIT_RECON_ALLOW_PRIVATE=1`.
- The delivery-test send path goes through the same AUP gate as campaign sends.
- Lab SMTP can speak cleartext (`smtp_security=plain`) or accept a private/self-
  signed relay cert (`accept_invalid_certs`), plus optional `PHISHKIT_SMTP_CA_FILE`.

### Added

- `make demo-cookie-tls` — serves the cookie app as an HTTPS upstream for the
  AiTM dry-run (self-signed cert under `demos/.certs/`).
- `EVILGINX_HTTPS_PORT` / `EVILGINX_DNS_PORT` — run the lure on unprivileged high
  ports without root.
- `tests/python/` — unit tests for the lure launcher (port selection, bind poll,
  config and lure-path logic).
- `tests/smoke/demo_cookie_e2e.sh` (`make test-aitm`) — unprivileged smoke test of
  the HTTPS upstream and cookie capture surface.
- `tests/smoke/aitm_e2e_sudo.sh` — full proxy loop through evilginx for a lab box.
- CI job that runs the Python unit tests and the AiTM smoke test.

### Changed

- `make setup` installs `pexpect` into a managed `venv/` and checks for `screen`.
- [Walkthrough](docs/guide/walkthrough.md) documents the AiTM demo end to end.

## [0.1.0] - 2026-08-14

**Alpha.** First tagged cut — primitive, not a beta, not production. We are
still assembling the product. Adds an isolated test harness so contributors can
run quality gates without touching a live operator database or stealing the
desktop. Walkthrough MP4s are generated locally only; they are not uploaded to
GitHub.

### Added

- `tests/unit/` — Rust public-API tests (assessments, mail import, phishlets, CLI)
- `#[cfg(test)]` helpers in `phishkit-core` (hosts, kit root, setup paths, tags)
- `tests/integration/` — WebdriverIO desktop suite covering Setup, Assessments,
  mail, campaigns, targets, sessions, and Settings
- `make test-integration-docker` — Linux Tauri + Xvfb; no host windows or app-data
- [Testing](docs/guide/testing.md) guide for operators and contributors

### Changed

- Desktop test feature flag is `test-hooks` (was `e2e`); frontend `VITE_TEST_HOOKS`
- Docs video remux uses `VIDEO=1` on the integration suite
- Destinations mailbox check is `make test-destinations` (`TEST_EMAIL` / `TEST_PASSWORD`)

### Notes

- Alpha: expect breaking changes; build from source; unsigned binaries
- UI suite is not on the PR critical path yet; `make test` / CI remain the gate
- Do not publish walkthrough videos to GitHub Releases

## [0.0.1] - 2026-08-09

Initial public preview (before the 0.1.0 alpha tag). Source-first, unsigned builds.
For authorized security assessments only — see [authorized use](docs/guide/authorized-use.md).

### Added

- Desktop app (`apps/desktop/`) — Tauri + React assessment workspace
- Shared engine (`crates/phishkit-core/`) — assessments, mail, campaigns, evilginx control, sessions
- Headless CLI (`apps/cli/`) — `phishkit` / `phishkit_ctl` mirroring desktop paths
- Guided CLI wizards — `phishkit wiz quickstart|send|sessions`
- TypeScript practice apps under `demos/` (`cookie`, `firebase`) + untun tunnel helper
- Vendored evilginx2 submodule + community phishlet packs under `vendor/`
- Packaging stubs (Homebrew, AUR, Debian) and tag-triggered release workflow
- Docs site (VitePress) with authorized-use, threat model, and operator guides

### Notes

- Pre-alpha: expect breaking changes; prefer building from source
- Packaging channel publish (tap / AUR / Launchpad) may require operator secrets
