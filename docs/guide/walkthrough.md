# Walkthrough videos

Recorded tours of the supported desktop app and the localhost demos. Use them
alongside the [quick start](/guide/quick-start). The same automation is an
**assertion suite** first — see [Testing](/guide/testing). Video is opt-in.

::: warning Authorized use only
phishkit is for written, in-scope assessments. The demos and recordings use
local mock apps only — never point automation at systems you are not authorized
to test. See [authorized use](/guide/authorized-use).
:::

## Desktop assessment walkthrough

Full console tour: create an Assessment, Overview lifecycle, Templates,
Recipients, Delivery presets, Campaigns (Guided / Composer / Express), Results,
Targets → cookie-session demo, Recon & Proxy, Sessions filters, and Settings.

<video controls playsinline preload="metadata" style="width:100%;max-width:960px;border-radius:8px;background:#0a0e15"><source src="https://github.com/openhat-security/phishkit/releases/download/docs-media/walkthrough-assessment.mp4" type="video/mp4" /><source src="https://github.com/openhat-security/phishkit/releases/download/docs-media/walkthrough-assessment.webm" type="video/webm" /></video>

## Cookie-session demo login

Cookie-session mock on `:9080` (`make demo-cookie`).

<video controls playsinline preload="metadata" style="width:100%;max-width:960px;border-radius:8px;background:#0a0e15"><source src="https://github.com/openhat-security/phishkit/releases/download/docs-media/walkthrough-demo-login.mp4" type="video/mp4" /><source src="https://github.com/openhat-security/phishkit/releases/download/docs-media/walkthrough-demo-login.webm" type="video/webm" /></video>

## Demo Firebase login

Firebase-shaped mock on `:9081` (`make demo-firebase`).

<video controls playsinline preload="metadata" style="width:100%;max-width:960px;border-radius:8px;background:#0a0e15"><source src="https://github.com/openhat-security/phishkit/releases/download/docs-media/walkthrough-demo-firebase.mp4" type="video/mp4" /><source src="https://github.com/openhat-security/phishkit/releases/download/docs-media/walkthrough-demo-firebase.webm" type="video/webm" /></video>

::: tip Videos not loading?
Media is published to the
[`docs-media`](https://github.com/openhat-security/phishkit/releases/tag/docs-media)
GitHub Release (not committed to git). If a player is blank, regenerate and upload:

```bash
make update-video-documentation
make publish-docs-videos    # requires gh auth + access to the docs-media release
```
:::

## How to record / update videos

Videos are **generated**, not edited by hand. They live under `docs/media/`
(gitignored). VitePress embeds stable GitHub Release URLs so the docs site stays
small.

The desktop tour is the integration suite with `VIDEO=1`. Prefer
`make test-integration-docker` for assertions without recording; use the
targets below when you need docs MP4s.

### Prerequisites

| Need | Why |
| --- | --- |
| Rust + Node 20+ | Build the desktop app and run WebdriverIO |
| Docker (preferred) | Isolate the UI so it does not steal the host display |
| `ffmpeg` (recommended) | Seekable MP4 output (`brew install ffmpeg`) |
| `gh` auth (publish only) | Upload to the `docs-media` Release |

### One command (everything)

```bash
make update-video-documentation
```

This:

1. Builds the React UI with `VITE_TEST_HOOKS=1` and a **release** Tauri binary
   with `--features test-hooks,custom-protocol` (embeds `frontendDist`; debug
   builds without `custom-protocol` stay on Vite `devUrl` and WebDriver sees
   `about:blank`).
2. Starts the cookie-session demo on `:9080`.
3. Runs WebdriverIO (`tests/integration`) with `VIDEO=1` in a data sandbox →
   remuxes into `docs/media/walkthrough-assessment.mp4`.
4. Starts both demos and records Playwright login clips →
   `walkthrough-demo-login.mp4` and `walkthrough-demo-firebase.mp4`.

### Piecewise targets

| Make target | What it records | Output |
| --- | --- | --- |
| `update-video-documentation-desktop` | Desktop console suite (WDIO, `VIDEO=1`) | `walkthrough-assessment.mp4` |
| `update-video-documentation-demos` | Cookie-session demo + Firebase logins (Playwright) | `walkthrough-demo-*.mp4` |
| `update-video-documentation` | Both of the above | all three |
| `publish-docs-videos` | Upload `walkthrough-*.mp4` only | Release tag `docs-media` |

### Publish to docs

```bash
make publish-docs-videos
# or point at another repo:
GITHUB_REPOSITORY=owner/repo make publish-docs-videos
```

Only files matching `docs/media/walkthrough-*.mp4` (and `.webm`) are uploaded.

### Where the automation lives

| Piece | Path |
| --- | --- |
| Desktop specs | `tests/integration/specs/*.spec.ts` |
| WDIO config | `tests/integration/wdio.conf.ts` |
| Demo login recorder | `scripts/demo_videos.py` |
| Release upload | `scripts/publish_docs_videos.sh` |
| Suite notes | `tests/integration/README.md` |

### What the suite does not do on video runs

Real SMTP send/test, admin `/etc/hosts` prompts, and Destinations mailbox
login. Archive / clone / restore **are** exercised because the suite uses a
throwaway sandbox, not your operator database.

See [Testing](/guide/testing) and [release notes](/reference/release).
