# Release process

phishkit is pre-1.0 (v0.1.x) software. This page describes how versions and
releases are handled today and the roadmap toward a signed stable release.

## Versioning

phishkit follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
User-facing changes are recorded in
[CHANGELOG.md](https://github.com/irruptio-security/phishkit/blob/main/CHANGELOG.md)
using the [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format. The
supported product is the desktop app; changes to the historical removed stacks are not tracked.

## Continuous integration

Every push and pull request to `main`/`staging` runs [CI](https://github.com/irruptio-security/phishkit/blob/main/.github/workflows/ci.yml):

- **Rust** (workspace: `phishkit-core`, `phishkit-cli`, desktop): `cargo fmt --check`, `cargo clippy
  --all-targets -D warnings`, and `cargo test --all-targets`, on macOS and Linux.
- **Frontend** (`apps/desktop`): `npm ci` + `npm run build`.
- **Workflow lint**: `actionlint` over the workflow files.

The [docs workflow](https://github.com/irruptio-security/phishkit/blob/main/.github/workflows/docs.yml)
builds the VitePress site and deploys it to GitHub Pages from `main`.

## Local quality gates

Before opening a pull request:

```bash
make test           # cargo fmt --check + cargo test (alias: make check)
make lint           # cargo clippy --all-targets
make docs-build     # when docs changed; fails on unresolved internal links
```

Desktop UI suite (optional, not on the PR critical path yet):

```bash
make test-integration-docker   # Linux + Xvfb; no host windows or app-data
```

See [Testing](/guide/testing).

## Docs walkthrough videos

VitePress embeds walkthrough MP4s from the dedicated GitHub Release tag
`docs-media` (binaries are **not** committed to git). Regenerate and publish:

```bash
make update-video-documentation           # desktop suite + demo logins
make update-video-documentation-desktop   # Tauri console suite only (VIDEO=1)
make update-video-documentation-demos     # Playwright demo logins only
make publish-docs-videos                  # gh release upload → docs-media
```

See [Walkthrough videos](/guide/walkthrough) and `tests/integration/README.md`.

## Building a release bundle

Build the desktop app bundle for your host platform with the Tauri CLI:

```bash
cd apps/desktop
npm ci
npm run tauri build
```

This produces a host-platform bundle under the workspace `target/` directory
(typically `target/release/bundle/`).

## Pre-1.0 signing and notarization roadmap

phishkit does not yet publish signed installers. Until it does:

- Builds are unsigned; macOS Gatekeeper and Windows SmartScreen will warn on
  first launch.
- Build from source, or verify any artifact you were given out-of-band before
  running it.

Planned before a `1.0.0` stable release:

- Signed, notarized macOS bundles and a signed Windows installer.
- Published checksums, an SBOM, and build provenance for release artifacts.
- Release CI that refuses a stable tag whose version or changelog section is
  missing or mismatched, and that produces a draft for maintainer review.

Because phishkit is offensive-security tooling that handles captured
credentials, treat every pre-stable build as sensitive and run it only in an
environment appropriate for an authorized engagement.
