# Tauri e2e (WebdriverIO) — docs desktop walkthrough

Official WebdriverIO + `@wdio/tauri-service` suite that drives the **real**
phishkit desktop binary (embedded WebDriver — works on macOS WKWebView) and
records the console tour used by the VitePress [walkthrough](../guide/walkthrough.md)
page.

## Prerequisites

- Rust toolchain + Node 20+
- `ffmpeg` recommended for seekable MP4
- Cookie-session demo: started automatically by Make (or `make demo-cookie`)

## Run (preferred)

From the kit root:

```bash
make update-video-documentation-desktop
# or the full docs video suite (desktop + demo logins):
make e2e
```

Legacy alias: `make e2e-tauri`.

### Manual

```bash
# Terminal A
make demo-cookie

# Terminal B
cd apps/desktop && VITE_E2E=1 npm run build
cargo build --release -p phishkit --features e2e,custom-protocol
cd docs/capture && npm ci && \
  PHISHKIT_E2E_BIN=../../target/release/phishkit npm test
```

Stable output: `docs/media/walkthrough-assessment.mp4` (gitignored).
Publish with `make publish-docs-videos`.

## Notes

- Build with `custom-protocol` so `frontendDist` is embedded. Without it, Tauri
  keeps `cfg(dev)` and loads `devUrl` (`http://localhost:1422`) — WebDriver sees
  `about:blank` if Vite is not running.
- The walkthrough is a **full console tour** (~4+ minutes wall time; stitched
  video is longer when interval frames + slowdown are enabled): Assessment
  create (all fields), Overview lifecycle, Templates, Recipients, Delivery
  presets, Campaigns (Guided / Composer / Express), Results, Target + Recon,
  Sessions, context chrome, Activity log. It intentionally skips real SMTP send
  and destructive archive/purge confirms.
- Non-e2e production builds omit plugin registration (`#[cfg(feature = "e2e")]`).
- Playwright demo login videos: `make update-video-documentation-demos`
  (`scripts/e2e_demo_videos.py`).

Full process docs: [docs/guide/walkthrough.md](../guide/walkthrough.md).
