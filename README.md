<h1 align="center">phishkit</h1>

<p align="center">
  <b>An authorized adversary-in-the-middle (AiTM) + awareness assessment platform.</b>
</p>

<p align="center">
  <a href="https://github.com/irruptio-security/phishkit/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/license-GPL--3.0-blue" alt="License: GPL-3.0" />
  </a>
  <a href="https://irruptio-security.github.io/phishkit/">
    <img src="https://img.shields.io/badge/docs-online-brightgreen" alt="Documentation" />
  </a>
</p>

phishkit is a local desktop app that wraps [evilginx](https://github.com/kgretzky/evilginx2)
(an AiTM proxy) together with a native email campaign engine into one
**authorized** end-to-end phishing workflow: **Assessment → Target →
Phishlet/Proxy → Lure → Template → Recipients → Campaign → Results → Session**.
It is deep enough that expert operators keep full control, and guided enough that
a non-technical business user can run a safe click-through campaign out of the
box.

**[Read the documentation →](https://irruptio-security.github.io/phishkit/)**

> **For authorized security assessments only.** phishkit drives an AiTM proxy,
> sends email, and handles captured credentials and live session tokens. Use it
> only with **explicit written authorization** from the owner of the targeted
> systems and people. Read [authorized use](docs/guide/authorized-use.md) and
> the [threat model](docs/reference/threat-model.md) before running anything.

## What you can do

- **Run one assessment end to end** — take an engagement from target setup and
  proxy to a captured session without stitching separate tools together, all in
  the desktop app.
- **Send with the native mail engine** — a draft → review → test → launch
  campaign composer bound to a target, named lure, sender, template, and
  recipient list; scheduling and send windows; delivered/opened/clicked/bounced
  tracking via **your** SMTP or ESP; and CSV/JSON/redacted reporting. Sender and
  content are snapshotted at creation for auditability.
- **Capture and attribute real sessions** — evilginx captures what an attacker
  would actually get, attributed deterministically back to the campaign attempt,
  with a focused Sessions view (timeline, masked credentials, token/cookie
  summary, export, and gated replay).
- **Serve two audiences** — a guided wizard and a curated preset scenario library
  with safe defaults for business users, layered over full Advanced controls for
  expert operators.
- **Run awareness campaigns** — a click-only training mode that never captures
  credentials.

## Languages

| Surface | Languages |
|---------|-----------|
| Product (desktop, CLI, engine, demos) | TypeScript + Rust |
| `scripts/` / kit glue | Python + shell OK |
| `vendor/evilginx2` | Go (upstream only) |

## Repository map

| Path | Role |
|------|------|
| `apps/desktop/` | Supported Tauri desktop app (React + Rust) |
| `apps/cli/` | Headless `phishkit` / `phishkit_ctl` |
| `crates/phishkit-core/` | Shared Rust engine |
| `kit/evilginx/` | Kit-owned phishlets, scripts, inject helpers |
| `demos/` | Localhost practice apps (`cookie`, `firebase`) |
| `docs/` | VitePress docs; `docs/capture/` → `docs/media/` (gitignored) |
| `vendor/` | `evilginx2` submodule + community phishlet packs |
| `scripts/` | Automation helpers (Python/shell) |
| `packaging/` | Homebrew / AUR / Debian stubs |

## Install

phishkit is **pre-alpha** (`v0.0.1`) software; build it from source and run it
locally. Use it only against domains and people you are authorized to assess.

```bash
git clone --recurse-submodules https://github.com/irruptio-security/phishkit.git
cd phishkit
make build          # ensure vendor sources, then build the evilginx binary
make setup          # rust check + desktop deps + docs deps
make start          # run the supported desktop app (tauri dev)
```

Requires git, Rust stable (`~/.cargo/bin` on `PATH`), Node (see `.nvmrc`), the Go
toolchain (to build evilginx once), and the Tauri prerequisites for your OS. Full
instructions, including platform support, are in the
[install guide](docs/guide/install.md).

The supported product is the desktop app under [`apps/desktop/`](apps/desktop/)
with the AiTM kit under [`kit/evilginx/`](kit/evilginx/).

## Command line

```bash
make cli
./target/release/phishkit --help
./target/release/phishkit wiz quickstart   # guided new assessment (TTY)
./target/release/phishkit list-assessments
```

See the [CLI guide](docs/guide/cli.md).

## Develop

Requires Node (see `.nvmrc`), Rust stable, Make, and the Tauri prerequisites for
your OS. `make help` lists every target.

```bash
make setup          # rust check + desktop deps + docs deps
make start          # run the desktop app
```

Quality checks before a PR (these mirror CI):

```bash
make check          # cargo fmt --check + cargo test
make lint           # cargo clippy --all-targets
```

Work on the documentation site:

```bash
make docs           # hot-reloading preview
make docs-build     # production build; fails on unresolved internal links
```

## Contributing

Contributions are welcome under GPL-3.0. Please read
[CONTRIBUTING.md](CONTRIBUTING.md) and our
[Code of Conduct](CODE_OF_CONDUCT.md) first. Report security issues privately per
[SECURITY.md](SECURITY.md) — never in a public issue. phishkit is for authorized
use; requests to enable unauthorized use are out of scope.

## Documentation

The full site is at
**[irruptio-security.github.io/phishkit](https://irruptio-security.github.io/phishkit/)**.

- [What phishkit is](docs/guide/index.md)
- [Authorized use](docs/guide/authorized-use.md)
- [Install](docs/guide/install.md)
- [Quick start](docs/guide/quick-start.md)
- [Campaign guide](docs/guide/campaigns.md)
- [Phishlet authoring](docs/guide/phishlets.md)
- [Command line](docs/guide/cli.md)
- [Architecture](docs/reference/architecture.md)
- [Platform support](docs/reference/platform-support.md)
- [Local data and network activity](docs/reference/data-and-network.md)
- [Threat model](docs/reference/threat-model.md)
- [Privacy](docs/reference/privacy.md)
- [Release process](docs/reference/release.md)
- [Changelog](CHANGELOG.md)
- [Security policy](SECURITY.md)

## License

phishkit is licensed under [GPL-3.0](LICENSE). It orchestrates and templates
around upstream open-source projects (notably
[evilginx2](https://github.com/kgretzky/evilginx2)); you are responsible for
understanding and complying with their licenses.

phishkit is an independent project. Use it lawfully and only with written
authorization.
