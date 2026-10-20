<h1 align="center">phishkit</h1>

<p align="center">
  <b>An authorized adversary-in-the-middle (AiTM) and awareness assessment platform.</b>
</p>

<p align="center">
  <a href="https://github.com/openhat-security/phishkit/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/license-GPL--3.0-blue" alt="License: GPL-3.0" />
  </a>
  <a href="https://openhat-security.github.io/phishkit/">
    <img src="https://img.shields.io/badge/docs-online-brightgreen" alt="Documentation" />
  </a>
  <a href="https://github.com/openhat-security/phishkit/releases/tag/v0.1.0">
    <img src="https://img.shields.io/badge/status-alpha-orange" alt="Alpha" />
  </a>
</p>

phishkit is a local desktop app that wraps [evilginx](https://github.com/kgretzky/evilginx2),
an AiTM proxy, together with a native email campaign engine so you can run one
**authorized** phishing assessment end to end: Assessment → Target →
Phishlet/Proxy → Lure → Template → Recipients → Campaign → Results → Session.
Expert operators keep full control through the Advanced panels, and a guided
wizard with safe presets lets a non-technical user run a click-through awareness
campaign.

**[Read the documentation →](https://openhat-security.github.io/phishkit/)**

> **Alpha (`v0.1.0`).** This is an early, in-progress release. Expect breaking
> changes, build from source, and note that there is no signed installer yet.
> The email campaign and tracking flow works today; the evilginx AiTM capture
> flow still needs manual setup to run (see [Run the AiTM proxy](#run-the-aitm-proxy)).

> **For authorized security assessments only.** phishkit drives an AiTM proxy,
> sends email, and handles captured credentials and live session tokens. Run it
> only with **explicit written authorization** from the owner of the targeted
> systems and people. Read [authorized use](docs/guide/authorized-use.md) and
> the [threat model](docs/reference/threat-model.md) first.

## What you can do

- Take one engagement from target setup through to a captured session inside the
  desktop app, without stitching separate tools together.
- Send with the native mail engine: a draft, review, test, and launch composer
  bound to a target, named lure, sender, template, and recipient list, with send
  windows, delivered/opened/clicked/bounced tracking through your own SMTP or
  ESP, and CSV/JSON/redacted reporting. The sender and content are snapshotted at
  creation so a campaign stays auditable.
- Capture and attribute real sessions: evilginx records what an attacker would
  actually get, tied back to the campaign attempt, with a Sessions view that
  shows the timeline, masked credentials, a token/cookie summary, export, and
  gated replay.
- Run a click-only awareness mode that never captures credentials, for training
  campaigns.

## Requirements

You need git, Rust stable (with `~/.cargo/bin` on your `PATH`), Node (see
`.nvmrc`), the Go toolchain to build evilginx once, and the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

The AiTM proxy needs two more things that the base setup does not install, so if
you plan to use evilginx, install them now:

```bash
# Debian/Ubuntu example
sudo apt-get install -y screen
python3 -m pip install pexpect   # or: pip install -r scripts/requirements.txt
```

evilginx also binds ports 443 and 53, which require elevated privileges. See
[Run the AiTM proxy](#run-the-aitm-proxy) for how to grant them.

## Install and run

phishkit is alpha. Build it from source and run it locally, only against domains
and people you are authorized to assess.

```bash
git clone --recurse-submodules https://github.com/openhat-security/phishkit.git
cd phishkit
make build          # fetch vendor sources, then build the evilginx binary
make setup          # rust check + desktop deps + docs deps
make start          # run the desktop app (tauri dev)
```

Requires git, Rust stable (`~/.cargo/bin` on `PATH`), Node (see `.nvmrc`), the Go
toolchain (to build evilginx once), and the Tauri prerequisites for your OS. The
AiTM lure also needs `screen` and Python `pexpect`, which `make setup` installs
into a managed `venv/`. Full instructions, including platform support, are in the
[install guide](docs/guide/install.md).

To run the bundled AiTM capture end to end against the cookie demo, follow
[AiTM demo end to end](docs/guide/walkthrough.md#aitm-demo-end-to-end). A quick
unprivileged check of the capture surface:

```bash
make test-aitm      # stands up the HTTPS upstream and drives the cookie login flow
```

The supported product is the desktop app under [`apps/desktop/`](apps/desktop/)
with the AiTM kit under [`kit/evilginx/`](kit/evilginx/).

This path uses the headless CLI and the bundled demo apps, and it runs without
elevated privileges. It exercises everything except the live evilginx capture.

```bash
make cli
B=./target/release/phishkit

# a sandbox so you don't touch your real data
export PHISHKIT_CONFIG=$PWD/.pk/config PHISHKIT_DATA=$PWD/.pk/data

make demo-cookie    # a mock login app on 127.0.0.1:9080 (run in another shell)

$B wiz quickstart           # guided new assessment (needs a TTY)
$B list-assessments
$B --help                   # every subcommand
```

From there you create a target, add an SMTP mail account, author a template, add
recipients, create and review a campaign, and export the report. The
[CLI guide](docs/guide/cli.md) and [walkthrough](docs/guide/walkthrough.md) cover
each step.

Sending real mail needs an SMTP server that presents a publicly trusted
certificate; the current build validates TLS against bundled roots and has no
plaintext or private-CA option, so a self-signed lab sink will be rejected. To
exercise the delivered/opened/clicked funnel without a live mail server, import
tracking events for a campaign and read the report.

## Run the AiTM proxy

The evilginx capture flow needs the extra setup from [Requirements](#requirements)
plus permission to bind privileged ports. Grant the ports once:

```bash
sudo setcap 'cap_net_bind_service=+ep' kit/evilginx/run/evilginx
```

Then drive a lure from the CLI:

```bash
$B ensure-destination ...        # scaffold a phishlet for your destination
$B start-lure -p <profile> -d <host> -P <phishlet>
```

The launcher runs evilginx inside a detached `screen` session named
`phishkit-evilginx`; attach with `screen -r phishkit-evilginx` and stop it with
`screen -S phishkit-evilginx -X quit`. evilginx proxies the lure host to the
real upstream site, so a working phishlet points at a reachable HTTPS origin that
is **not** the lure host. The bundled `demo-cookie` phishlet is a capture-surface
template against an HTTP-only mock, so it illustrates the config but does not
complete a live proxy on its own. See [phishlet authoring](docs/guide/phishlets.md)
for building one that captures.

## Repository map

| Path | Role |
|------|------|
| `apps/desktop/` | Supported Tauri desktop app (React + Rust) |
| `apps/cli/` | Headless `phishkit` / `phishkit_ctl` |
| `crates/phishkit-core/` | Shared Rust engine |
| `kit/evilginx/` | Kit-owned phishlets, scripts, inject helpers |
| `demos/` | Localhost practice apps (`cookie`, `firebase`) |
| `docs/` | VitePress docs; generated videos in `docs/media/` (gitignored) |
| `tests/` | `unit/` (Rust) and `integration/` (desktop UI + Docker) |
| `vendor/` | `evilginx2` submodule + community phishlet packs |
| `scripts/` | Automation helpers (Python/shell) |
| `packaging/` | Homebrew / AUR / Debian stubs |

## Languages

| Surface | Languages |
|---------|-----------|
| Product (desktop, CLI, engine, demos) | TypeScript + Rust |
| `scripts/` and kit glue | Python + shell |
| `vendor/evilginx2` | Go (upstream only) |

## Develop

`make help` lists every target. The checks below mirror CI:

```bash
make test           # cargo fmt --check + cargo test (core + cli)
make lint           # cargo clippy --all-targets
make test-integration-docker   # desktop UI suite in Linux + Xvfb (optional)
```

Do not run the desktop UI suite against your live Application Support database.
See [Testing](docs/guide/testing.md).

Work on the documentation site:

```bash
make docs           # hot-reloading preview
make docs-build     # production build; fails on unresolved internal links
```

## Contributing

Contributions are welcome under GPL-3.0. Read
[CONTRIBUTING.md](CONTRIBUTING.md) and the
[Code of Conduct](CODE_OF_CONDUCT.md) first. Report security issues privately per
[SECURITY.md](SECURITY.md) rather than in a public issue. phishkit is for
authorized use, and requests to enable unauthorized use are out of scope.

## Documentation

The full site is at
**[openhat-security.github.io/phishkit](https://openhat-security.github.io/phishkit/)**.

- [What phishkit is](docs/guide/index.md)
- [Authorized use](docs/guide/authorized-use.md)
- [Install](docs/guide/install.md)
- [Quick start](docs/guide/quick-start.md)
- [Testing](docs/guide/testing.md)
- [Walkthrough](docs/guide/walkthrough.md)
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
around upstream open-source projects, notably
[evilginx2](https://github.com/kgretzky/evilginx2); you are responsible for
understanding and complying with their licenses.

phishkit is an independent project. Use it lawfully and only with written
authorization.
