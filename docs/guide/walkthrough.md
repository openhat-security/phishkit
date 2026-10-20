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

## AiTM demo end to end

The cookie demo can run as a full evilginx dry-run: a "victim" opens a lure URL,
evilginx proxies to the real app, and the session cookie is captured. This needs
two distinct hosts, because an AiTM proxy cannot proxy to itself (the shipped
phishlet used to point its upstream at the lure host, so evilginx dialed its own
hostname and looped).

| Host | Role | Resolves to |
| --- | --- | --- |
| `demo-cookie.local.phishkit` | Lure, served by evilginx | `127.0.0.1` |
| `app.cookie-demo.test` | Upstream app evilginx proxies to | `127.0.0.2` |

The shipped `demo-cookie` phishlet is wired for this split.

### 1. Install the runtime deps

`make setup` installs `pexpect` into a managed `venv/` and checks for `screen`.
evilginx needs `screen` to run with a real TTY; install it if setup reports it
missing (`apt-get install screen` or `brew install screen`).

### 2. Map the hosts

```bash
sudo tee -a /etc/hosts <<'EOF'
127.0.0.1   demo-cookie.local.phishkit
127.0.0.2   app.cookie-demo.test
EOF
```

### 3. Start the HTTPS upstream

```bash
sudo make demo-cookie-tls        # serves the cookie app over TLS on 127.0.0.2:443
```

This generates a self-signed cert for `app.cookie-demo.test` (under
`demos/.certs/`, gitignored) and serves the cookie app over HTTPS. evilginx
proxies to it with certificate verification off, so a self-signed cert is fine
for the dry-run.

### 4. Launch the lure

evilginx binds `:443` and `:53`, which need privileges. Grant the binary the
capability once, or run as root:

```bash
sudo setcap cap_net_bind_service=+ep kit/evilginx/run/evilginx
```

Then start the lure from the desktop app (Recon & Proxy, then Start + get lure,
phishlet `demo-cookie`) or the CLI. Open the printed
`https://demo-cookie.local.phishkit/...` URL in a fresh browser profile and click
through the self-signed warning. Sign in with `demo@phishkit.local` /
`demo-password`; the captured session shows up in Sessions.

#### Unprivileged lure port

To avoid root for the lure, set high ports before starting:

```bash
export EVILGINX_HTTPS_PORT=8443
export EVILGINX_DNS_PORT=5353
```

The lure then serves on `:8443`. evilginx still dials the upstream on `:443`, so
`make demo-cookie-tls` keeps needing root or setcap for that listener.

### Teardown

The launcher tears down its own `screen` session when a start fails. To stop a
running lure:

```bash
make evilginx-stop               # quits the phishkit-evilginx screen session
```

### Automated checks

```bash
make test-aitm                           # unprivileged: HTTPS upstream + cookie capture surface
make test-python                         # lure launcher unit tests
sudo bash tests/smoke/aitm_e2e_sudo.sh   # full proxy loop through evilginx (lab box)
```
