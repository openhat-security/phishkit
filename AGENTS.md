# phishkit contributor / agent conventions

## Product direction

- The supported product is the local Tauri/React/Rust app under `apps/desktop/`.
- Run it with `make desktop`. Kit assets live under `kit/evilginx/`; evilginx is built from `vendor/evilginx2`.
- Shared engine code belongs in `crates/phishkit-core/`. The desktop `src-tauri` crate and `apps/cli` are thin frontends over that core.
- Email delivery is owned by the native Rust engine (`mail`, `campaign`); do not reintroduce Gophish.
- Mutable operator data belongs in OS app-data / config dirs (or an ephemeral sandbox), not under the git checkout `run/` tree.

## Languages

| Surface | Languages |
|---------|-----------|
| **Product source** | **TypeScript** — `apps/desktop/src`, `demos/`, `docs/capture` · **Rust** — `crates/phishkit-core`, `apps/desktop/src-tauri`, `apps/cli` |
| **Scripts / kit glue** | **Python and shell OK** under `scripts/` and `kit/evilginx/scripts/` |
| **Go** | Only vendored `vendor/evilginx2` and its build |

Do not add Python (or other languages) inside app/UI/engine crates. Prefer Rust/TS when a script becomes a product feature (Tauri command / `phishkit` subcommand).

## Demos

Practice apps live under `demos/` (`cookie`, `firebase`, community notes). Use `make demo-*` and optional `make demo-tunnel PORT=…` (untun / Cloudflare quick tunnel, no signup). Authorized lab use only.

## Authorized use

phishkit is for **authorized security assessments only**. Do not add features whose purpose is unauthorized phishing or evasion of lawful controls. See `docs/guide/authorized-use.md`.
