# phishkit — Usage (end to end)

Authorized engagements only. The local dry-run validates your phishlet against
**yourself**; real victim testing needs a VPS + registered lookalike domain.

### Preferred: desktop (AiTM + native mail)

```bash
make build-evilginx          # once
make demo-cookie             # optional — cookie mock on :9080 (see demos/)
make desktop                 # Tauri GUI:
                             #   Assessments → Targets → Phishlet/Proxy → Lures →
                             #   Templates → Recipients → Campaigns → Results → Sessions
```

Flow: create a Target / tracked lure → HTML template with `{{link}}` → CSV recipients →
Delivery (SMTP or ESP) → accept authorized-use AUP → send campaign → sync captures.

Community packs ship under `vendor/community-phishlets/`; `make community-phishlets`
refreshes pins. See [`demos/`](demos/) and [`apps/desktop/README.md`](apps/desktop/README.md).

Guided CLI: `make cli && ./target/release/phishkit wiz quickstart` (authorized use only).

---

## 0. Build (one time)

```bash
git submodule update --init --recursive   # if clone omitted --recurse-submodules
make build          # init submodules + build evilginx2 binary
make setup          # desktop + docs deps
```

---

## 0b. Localhost demos (first-run practice)

```bash
make demo-cookie                              # http://127.0.0.1:9080
make validate-phishlet PHISHLET=demo-cookie
# or
make demo-firebase                            # http://127.0.0.1:9081
make validate-phishlet PHISHLET=demo-firebase
```

Creds: `demo@phishkit.local` / `demo-password`. Target notes:
[`demos/cookie/`](demos/cookie/),
[`demos/firebase/`](demos/firebase/).

---

## 1. Scaffold a phishlet for the target

```bash
make campaign TARGET=app.client.com
# edit the generated kit/evilginx/phishlets/<...>.yaml if needed, then:
make validate-phishlet PHISHLET=<phishlet-name>
```

Or start from `kit/evilginx/phishlets/generic.yaml` / a template under
`evilginx/phishlet-templates/`. `proxy_hosts.domain` and `login.domain` must be
the **real** authorized target domain. The dry-run hostname is applied at
runtime, never baked into the phishlet.

---

## 2. Start evilginx (local dry-run) and get the lure URL

Defaults use the cookie-session demo:

```bash
# add /etc/hosts entries so the dry-run host resolves to 127.0.0.1
sudo tee -a /etc/hosts <<EOF
127.0.0.1   demo-cookie.local.phishkit
EOF

make evilginx-start          # PHISHLET_NAME=demo-cookie by default
screen -r phishkit-evilginx
```

At the evilginx `:` prompt paste the block printed by the launcher (domain +
`phishlets hostname` + lure). Prefer driving the same flow from **Destinations**
in `make desktop`.

---

## 3. Send with the desktop mail engine

Configure a sender in the desktop app (SMTP or ESP HTTP API), compose a campaign
bound to the Target / named lure / template / recipient list, then
**Review → Test → Launch**. Delivery is owned by the native Rust engine.

---

## 4. View captured credentials + tokens

In the app: **Sessions** (timeline, masked credentials, export, gated replay).

CLI / scripts:

```bash
make evilginx-creds                          # summary
./evilginx/scripts/view_creds.sh --full      # full token values
./evilginx/scripts/view_creds.sh --json      # raw json
```

Cookie demos yield a `session` cookie; Firebase demos yield
`custom.{refresh_token,id_token,local_id}`.

---

## 5. Teardown

```bash
make evilginx-stop
make clean             # removes run/ (binaries, data.db, logs)
```

Export any captures to a secure location first. Captured passwords + tokens are
incident-response-grade sensitive.

---

## One-shot local test

```bash
make quick-test TARGET=app.client.com EMAIL=you@yourdomain.com
```

Or Destinations mailbox test (authorized mailbox required):

```bash
TEST_EMAIL='you@example.com' TEST_PASSWORD='…' make test-destinations
# default TEST_TARGET=demo-cookie.local.phishkit
```

---

## Removed surfaces

- Docker Compose hand-off (`docker/`) is gone — use the desktop app or CLI.
