# Walkthrough videos

Recorded tours of the supported desktop app and the localhost demos. Use them
alongside the [quick start](/guide/quick-start).

::: warning Authorized use only
phishkit is for written, in-scope assessments. The demos and recordings use
local mock apps only — never point automation at systems you are not authorized
to test. See [authorized use](/guide/authorized-use).
:::

## Desktop assessment walkthrough

Full console tour at a followable pace: create an Assessment (all fields), Overview
lifecycle, Templates, Recipients, Delivery presets, Campaigns (Guided / Composer /
Express), Results, Targets → cookie-session demo, Recon & Proxy, Sessions filters, context
switchers, and the Activity log. Does not send real mail or confirm destructive
purge/archive.

<video controls playsinline preload="metadata" style="width:100%;max-width:960px;border-radius:8px;background:#0a0e15"><source src="https://github.com/irruptio-security/phishkit/releases/download/docs-media/walkthrough-assessment.mp4" type="video/mp4" /><source src="https://github.com/irruptio-security/phishkit/releases/download/docs-media/walkthrough-assessment.webm" type="video/webm" /></video>

## Cookie-session demo login

Cookie-session mock on `:9080` (`make demo-cookie`).

<video controls playsinline preload="metadata" style="width:100%;max-width:960px;border-radius:8px;background:#0a0e15"><source src="https://github.com/irruptio-security/phishkit/releases/download/docs-media/walkthrough-demo-login.mp4" type="video/mp4" /><source src="https://github.com/irruptio-security/phishkit/releases/download/docs-media/walkthrough-demo-login.webm" type="video/webm" /></video>

## Demo Firebase login

Firebase-shaped mock on `:9081` (`make demo-firebase`).

<video controls playsinline preload="metadata" style="width:100%;max-width:960px;border-radius:8px;background:#0a0e15"><source src="https://github.com/irruptio-security/phishkit/releases/download/docs-media/walkthrough-demo-firebase.mp4" type="video/mp4" /><source src="https://github.com/irruptio-security/phishkit/releases/download/docs-media/walkthrough-demo-firebase.webm" type="video/webm" /></video>

::: tip Videos not loading?
Media is published to the
[`docs-media`](https://github.com/irruptio-security/phishkit/releases/tag/docs-media)
GitHub Release (not committed to git). If a player is blank, regenerate and upload:

```bash
make e2e                    # or: make update-video-documentation
make publish-docs-videos    # requires gh auth + access to the docs-media release
```
:::

## How to record / update videos

Videos are **generated**, not edited by hand. They live under `docs/media/`
(gitignored). VitePress embeds stable GitHub Release URLs so the docs site stays
small.

### Prerequisites

| Need | Why |
| --- | --- |
| Rust + Node 20+ | Build the desktop app and run WebdriverIO |
| `ffmpeg` (recommended) | Seekable MP4 output (`brew install ffmpeg`) |
| `gh` auth (publish only) | Upload to the `docs-media` Release |
| macOS for the desktop tour | Embedded WebDriver targets WKWebView |

### One command (everything)

```bash
make e2e
# equivalent:
make update-video-documentation
```

This:

1. Builds the React UI with `VITE_E2E=1` and a **release** Tauri binary with
   `--features e2e,custom-protocol` (embeds `frontendDist`; debug builds without
   `custom-protocol` stay on Vite `devUrl` and WebDriver sees `about:blank`).
2. Starts the cookie-session demo on `:9080`.
3. Runs WebdriverIO (`docs/capture`) against the real binary →
   `docs/media/walkthrough-assessment.mp4`.
4. Starts both demos and records Playwright login clips →
   `walkthrough-demo-login.mp4` and `walkthrough-demo-firebase.mp4`.

### Piecewise targets

| Make target | What it records | Output |
| --- | --- | --- |
| `update-video-documentation-desktop` | Full desktop console tour (WDIO) | `walkthrough-assessment.mp4` |
| `update-video-documentation-demos` | Cookie-session demo + Firebase logins (Playwright) | `walkthrough-demo-*.mp4` |
| `update-video-documentation` / `e2e` | Both of the above | all three |
| `publish-docs-videos` | Upload `walkthrough-*.mp4` only | Release tag `docs-media` |

Legacy aliases still work: `e2e-tauri`, `docs-videos`, `docs-videos-demos`.

### Publish to docs

```bash
make publish-docs-videos
# or point at another repo:
GITHUB_REPOSITORY=owner/repo make publish-docs-videos
```

Only files matching `docs/media/walkthrough-*.mp4` (and `.webm`) are uploaded,
so WDIO run leftovers are not published.

### Where the automation lives

| Piece | Path |
| --- | --- |
| Desktop tour spec | `docs/capture/specs/walkthrough.e2e.ts` |
| WDIO config (slowdown, interval frames) | `docs/capture/wdio.conf.ts` |
| Demo login recorder | `scripts/e2e_demo_videos.py` |
| Release upload | `scripts/publish_docs_videos.sh` |
| Suite notes | `docs/capture/README.md` |

### Tuning the desktop tour

- Dwells and character-by-character typing live in the walkthrough spec so viewers
  can follow form fill.
- `wdio.conf.ts` uses `screenshotIntervalSecs` so pauses appear in the stitched
  video, plus `videoSlowdownMultiplier` for readability.
- Prefer `ffmpeg` remux after WDIO — the Makefile does this so the MP4 has a
  complete `moov` atom (seekable in browsers).

### What the tour intentionally skips

Real SMTP send/test, admin `/etc/hosts` prompts, and confirming purge/archive/
delete dialogs. Archive keeps data inactive; Delete erases the engagement from
the app DB. Both are available in Assessments (Show archived) and Overview →
End assessment.

See also [demos/](https://github.com/irruptio-security/phishkit/tree/main/examples)
and [release notes](/reference/release).
