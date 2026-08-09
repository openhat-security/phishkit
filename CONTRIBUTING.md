# Contributing

phishkit is GPL-3.0 software. By contributing, you agree that your contribution
is distributed under GPL-3.0. Participation is governed by our
[Code of Conduct](CODE_OF_CONDUCT.md).

phishkit is an **authorized-assessment** tool. Do not contribute features,
presets, phishlets, or documentation whose primary purpose is to enable
unauthorized use, evade lawful detection, or remove the authorized-use gate.
See [authorized use](docs/guide/authorized-use.md).

## Languages

| Surface | Languages |
|---------|-----------|
| Product (`apps/`, `crates/`, `demos/`, `docs/capture`) | TypeScript + Rust |
| Automation (`scripts/`, `kit/evilginx/scripts/`) | Python + shell OK |
| Vendored proxy | Go only in `vendor/evilginx2` |

Do not add Python inside app/UI/engine crates. Prefer Rust CLI/Tauri commands when
automation becomes a product feature.

Practice apps live under `demos/` (`cookie`, `firebase`).

## Supported surface

The supported product is [`apps/desktop/`](apps/desktop/), driven by `make desktop`.
Shared engine: [`crates/phishkit-core/`](crates/phishkit-core/). CLI:
[`apps/cli/`](apps/cli/).

## Development

```bash
make setup          # rust toolchain check + desktop npm install + docs deps
make check          # cargo fmt --check + cargo test (core + cli)
make lint           # cargo clippy (workspace packages)
make cli            # release CLI binaries
make desktop        # tauri dev
```

`make help` lists the full target set.

## Docs

```bash
make docs           # VitePress preview
make docs-build     # production build (fails on broken internal links)
```

Docs live under `docs/` and deploy to GitHub Pages from `main`. Prefer
`docs/guide/` and `docs/reference/` for operator content.

## Pull requests

- Keep changes focused; match existing style.
- Do not commit secrets, captures, `run/` state, or `node_modules/`.
- Keep the AUP/authorized-use gate and allow-listed session replay intact.
