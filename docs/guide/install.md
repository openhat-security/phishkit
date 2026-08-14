# Install

phishkit is pre-1.0 alpha software. Build it from source and run it locally.

## Prerequisites

- **git** (the project uses submodules under `vendor/`).
- **Rust** stable, with `~/.cargo/bin` on your `PATH` (install via
  [rustup](https://rustup.rs)).
- **Node.js** — see [`.nvmrc`](https://github.com/openhat-security/phishkit/blob/main/.nvmrc)
  (Node 20+). `nvm use` picks it up.
- **Go** toolchain — required once to build the bundled evilginx binary.
- A **Tauri** toolchain for your OS (WebView + build tools). See the
  [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

## Get the source

Clone with submodules so `vendor/evilginx2` is populated:

```bash
git clone --recurse-submodules https://github.com/openhat-security/phishkit.git
cd phishkit
```

Already cloned without submodules?

```bash
git submodule update --init --recursive   # or: make vendor
```

## Build and set up

```bash
make build          # ensure vendor sources, then build the evilginx binary
make setup          # rust check + desktop npm install + docs deps
```

`make help` lists every target.

## Run the desktop app

```bash
make desktop        # npm install + tauri dev
# equivalently:
make start
```

The app opens the assessment workspace. If kit discovery fails in a
non-standard checkout, set `PHISHKIT_ROOT` to the repository root.

## Platform support

phishkit's operator workflow is developed and tested primarily on **macOS**.
Elevated `/etc/hosts` edits (and their paired cleanup) use macOS admin
prompts. Linux is supported for building and running; some elevated helpers are
macOS-specific. See [platform support](/reference/platform-support) for the
current matrix.

## Documentation site (optional)

To work on these docs locally:

```bash
make docs           # hot-reloading preview
make docs-build     # production build; fails on unresolved internal links
```


## First launch

On first launch the desktop app opens a **Setup** wizard: storage mode (persistent OS app-data vs ephemeral temp sandbox), persona, and an optional tutorial. Preferences are stored in the OS config directory as `setup.json`.

CLI users can point at the same paths with `PHISHKIT_DATA` / `PHISHKIT_CONFIG` / `PHISHKIT_ROOT`.
