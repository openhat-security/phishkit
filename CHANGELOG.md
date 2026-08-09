# Changelog

All notable changes to phishkit are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.1] - 2026-08-09

Initial **pre-alpha** public preview (before beta). Source-first, unsigned builds.
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
