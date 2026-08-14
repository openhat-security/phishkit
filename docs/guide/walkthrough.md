# Walkthrough

A text tour of the desktop console. Use it with the [quick start](/guide/quick-start).
The same flows are covered by the [integration suite](/guide/testing).

::: warning Alpha — no hosted videos
phishkit is an **alpha**. We do **not** upload walkthrough MP4s to GitHub
Releases. Multi-minute recordings grow quickly, count against GitHub bandwidth,
and do not belong in git. Generate clips locally if you need them (below).
A later release may host compressed shorts on a CDN — not as GitHub assets.
:::

::: warning Authorized use only
The demos and recordings use local mock apps only — never point automation at
systems you are not authorized to test. See [authorized use](/guide/authorized-use).
:::

## Console tour (what you will see)

1. **Setup** — storage mode, persona, skip or take the in-app tour.
2. **Assessments** — create an engagement (name, primary domain, authorization).
3. **Overview** — export, clone, archive / restore. Destructive purge/delete
   exist; the automated suite only confirms them in a throwaway sandbox.
4. **Templates / Recipients / Delivery** — starter template, paste a list, pick
   a sender preset. Alpha does not require a real SMTP send to explore the UI.
5. **Campaigns** — Guided, Composer (AUP), Express.
6. **Results** — funnel and optional event import.
7. **Targets** — add the cookie-session demo at `http://127.0.0.1:9080`.
8. **Recon & Proxy / Sessions** — destination steps, filters, search.

Practice apps: `make demo-cookie` (`:9080`) and `make demo-firebase` (`:9081`).

## Record locally (optional)

Assertions are the default. Video is opt-in and stays on your machine
(`docs/media/` and `tests/integration/artifacts/`, both gitignored).

```bash
make test-integration-docker          # no video, no host windows
VIDEO=1 make test-integration         # local MP4s (opens a host window)
make update-video-documentation       # desktop + demo login clips → docs/media/
```

`make publish-docs-videos` is disabled. If we host shorts later, they will
live on a CDN or unlisted YouTube — not as GitHub Release assets.

| Need | Why |
| --- | --- |
| Rust + Node 20+ | Build the desktop app and run WebdriverIO |
| Docker (preferred) | Isolate the UI so it does not steal the host display |
| `ffmpeg` (recommended) | Seekable MP4 output (`brew install ffmpeg`) |

See [Testing](/guide/testing) and [release notes](/reference/release).
