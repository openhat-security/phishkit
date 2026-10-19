# PhishKit — Security & Functionality Audit

**Repo:** https://github.com/openhat-security/phishkit (openhat-security)
**Audited on:** the Grok Bot box (Debian 13 x86_64, headless), not any registered machine.
**Scope:** build from source, run and use every main path end to end, security review.
**Method:** CLI driven in a sandbox (`PHISHKIT_CONFIG=/tmp/pk-sb/config`, `PHISHKIT_DATA=/tmp/pk-sb/data`), local demo targets only. No third-party services were touched.

> Convention: Critical / High / Medium / Low. Each finding lists evidence, impact, and a fix. Anything I could not fully verify is marked **(unconfirmed)**.

---

## 1. What PhishKit is

A Tauri 2 desktop app plus a `phishkit` headless CLI for running authorized adversary-in-the-middle (AiTM) phishing engagements. A red-teamer defines an *assessment* and *targets*, authors email *templates*, configures an SMTP *mail account*, sends lures to *recipients*, and (the headline feature) launches a bundled **evilginx2** reverse proxy via a generated *phishlet* to capture live credentials and session cookies. There is an AUP (acceptable-use policy) gate meant to keep it to authorized engagements, plus recon/detection helpers and a tracking/reporting funnel.

Tech: Rust core (`crates/phishkit-core`), React/Tauri frontend, SQLite store, `lettre` for SMTP, a vendored `evilginx2` Go submodule driven by Python (`pexpect` + `screen`), and two local demo "victim" apps (`demos/cookie` on :9080, `demos/firebase` on :9081).

## 2. Did it work? (verified)

| Path | Result |
|---|---|
| Build core + CLI (`cargo build --release`) | ✅ builds (Rust 1.99) |
| Build evilginx submodule (`make build-evilginx`, go1.27.2) | ✅ builds |
| Demo apps (`make demo-cookie`, `demo-firebase`) | ✅ serve on 127.0.0.1:9080 / :9081 (firebase needed `npm install` to finish first — see M-x) |
| Assessment → target → mail account → template → recipients → campaign | ✅ full CRUD works via CLI |
| Campaign review + `import-events` → tracking funnel → report export | ✅ end-to-end funnel works |
| Recon / `detect` against local demo | ✅ (`generic_spa /login`) — but see H3 (SSRF) |
| **AiTM lure (`start-lure` → evilginx proxy → capture)** | ❌ **does not complete out of the box** — see **C1** |
| Real SMTP byte delivery to a lab sink | ❌ no plaintext / private-CA path — see H5 |

**Bottom line:** the mail/campaign/tracking half works end to end. The core red-team feature — evilginx AiTM capture — does **not** work out of the box: it has undocumented runtime dependencies, needs elevated privileges, the launcher reports false failure, and the shipped demo phishlet is wired to proxy to itself so no session is ever captured.

---

## 3. Findings

### C1 — Critical: the AiTM capture flow cannot complete end to end out of the box
The headline feature fails as shipped, for a stack of independent reasons I verified in sequence:

1. **Undocumented runtime deps.** `start-lure` drives `kit/evilginx/scripts/configure_lure.py`, which needs `pexpect` and the `screen` binary. Neither is installed by `make setup`; `pexpect` is only listed in `scripts/requirements.txt` (not referenced by setup) and `screen` is nowhere. Out of the box: `FileNotFoundError: [Errno 2] No such file or directory: 'screen'`.
2. **Needs root / capabilities.** evilginx binds :443 and :53. As an unprivileged user it logs `listen tcp :443: bind: permission denied` and `Failed to start nameserver on: :53`. There is no documentation of this and no helper to grant it. (I worked around it with `setcap cap_net_bind_service=+ep` on `kit/evilginx/run/evilginx`, after which :443 bound.)
3. **Launcher false-negative.** Even once :443 bound successfully, the Rust wrapper still returned `"evilginx did not bind HTTPS port 443 in time"` while evilginx was in fact `LISTEN *:443` and running in the screen session. The bind poll races the process and reports failure on success. Evidence: `ss -tlnp` showed `LISTEN *:443` with the evilginx pid while `start-lure` returned the error JSON.
4. **Shipped phishlet proxies to itself.** `kit/evilginx/phishlets/demo-cookie.yaml` sets `proxy_hosts.domain`, `login.domain`, and `auth_tokens.domain` all to `demo-cookie.local.phishkit` — the *same* host as the lure. evilginx therefore dials its own hostname for the upstream and loops: `cert_db: could not clone upstream cert for demo-cookie.local.phishkit:443 (dial tcp 127.0.0.1:443: connect: cannot assign requested address) — using self-signed`, then `Cannot handshake client ... i/o timeout`. The phishlet's own header comment admits it: *"the mock speaks HTTP on :9080 … you still need a reachable HTTPS upstream … This phishlet is the capture-surface template"* — i.e. it is a template that cannot actually proxy or capture against the shipped HTTP-only demo.
5. **`ensure-destination` scaffolds a colliding phishlet.** It created a second phishlet (`demo-cookie-local-phishkit`) whose hostname collides with `demo-cookie`: `phishlets: hostname 'demo-cookie.local.phishkit' collision between 'demo-cookie' and 'demo-cookie-local-phishkit'` (logged 3×).

**Impact:** the product's primary function is non-functional for a new user. No credential/cookie is ever captured by the documented demo path. A red-teamer following the README reaches a dead end.
**Fix:** (a) have `make setup` install `pexpect` into a managed venv and install/document `screen`; (b) ship a privileged-port story (setcap on build, or a helper, or run evilginx on a high port behind the demo); (c) fix the wrapper bind-poll to re-check after the process is confirmed up before returning failure; (d) ship a *working* demo phishlet whose upstream points at a reachable HTTPS origin distinct from the lure host (give the demo an HTTPS listener or a second hostname), and make `ensure-destination` reuse the existing phishlet instead of scaffolding a colliding duplicate.

### H1 — High: SMTP credentials stored in plaintext at rest
`smtp_settings.password` is stored verbatim in SQLite. Verified: `strings /tmp/pk-sb/data/phishkit.db` shows the SMTP password in cleartext. `mail.rs`/display code only `mask_secret` for *display*; the stored value is plaintext.
**Impact:** anyone who reads the DB file (backup, sync, another local user — see H2) gets the mail-sending credential.
**Fix:** encrypt secrets at rest (OS keychain / libsecret, or an app-level key), or at minimum stop persisting the password and require it per-session.

### H2 — High: SQLite store is world-readable (0644)
`db.rs` opens/creates the DB with no `set_permissions`; the file lands `-rw-r--r--`. Verified: `/tmp/pk-sb/data/phishkit.db` is `0644`.
**Impact:** on a multi-user box every local user can read captured credentials, session cookies, recipient lists, and the plaintext SMTP password (H1). For a tool whose whole job is collecting live secrets this is a serious at-rest exposure.
**Fix:** create the data dir `0700` and the DB `0600` on first open (mirror the fix already applied to OnionGate's key files).

### H3 — High: SSRF in recon/`detect` with TLS verification disabled
`recon.rs` (~line 262) builds its HTTP client with `danger_accept_invalid_certs(true)` and fetches a user-supplied host, following redirects. I verified `detect` fetched an internal address (`127.0.0.1:8888`) with no allow-listing.
**Impact:** an operator (or anything that can set the target field) can make the app probe internal/cloud-metadata endpoints, and the disabled cert check means MitM/forged-cert responses are trusted. In a red-team tool this both leaks internal reachability and undermines the integrity of recon results.
**Fix:** verify TLS by default (make "accept invalid certs" an explicit per-target opt-in with a warning), and block/denylist link-local, loopback, and RFC1918 targets unless the engagement scope explicitly includes them.

### H4 — High: AUP gate is not enforced on the send-test path
The AUP gate (`aup.rs`, `require_aup`) that is meant to block action until the operator accepts the policy is checked on the main send path but not on `send-test`. **(Partially unconfirmed on the GUI path — verified in CLI code review that the test path does not call the gate.)**
**Impact:** the safety control that keeps this to authorized engagements can be skipped to send real mail through the "test" path.
**Fix:** route every send (including test) through the same AUP check.

### H5 — High (functionality): no plaintext / private-CA SMTP path for lab use
`lettre` is built with `rustls-tls`, which uses bundled `webpki-roots` and ignores `SSL_CERT_FILE`/system trust. There is no plaintext-SMTP or custom-CA option, so a local/lab mail sink (self-signed or private CA) cannot be used: delivery fails with `CaUsedAsEndEntity` / `UnknownIssuer`. Verified against a local SMTP sink.
**Impact:** you can only send through a public ESP with a publicly-trusted cert — exactly what you often cannot or should not do in a closed lab exercise. Contradicts the "dry-run / local demo" positioning.
**Fix:** add a native-TLS or "danger: accept invalid cert for this relay" option and/or a plaintext/STARTTLS-optional mode gated to lab relays, and honor a custom CA bundle.

### M1 — Medium: `upstream_domain` mishandles short/compound hosts
`engagement.rs::upstream_domain` (line 36) derives the upstream by stripping the left label, which is wrong for compound public suffixes (e.g. `*.co.uk`, `*.github.io`) and for single-label internal hosts. This feeds the broken phishlet generation in C1.
**Fix:** use a public-suffix-list–aware eTLD+1 computation.

### M2 — Medium: dependency CVEs (cargo audit: 9 vulnerabilities)
`cargo audit` reports 9 vulnerabilities and 11 allowed warnings. Notable: `rustls` (4 advisories), `quinn-proto` (5), `chacha20`, plus unmaintained `proc-macro-error`, `unic-*`, `zbus_names`/`zvariant`, `glib`. Several ride in through the Tauri/`reqwest` tree.
**Fix:** `cargo update` the rustls/quinn stack to patched versions; replace unmaintained crates; add `cargo audit` to CI (see M4).

### M3 — Medium: CI actions pinned to moving tags (supply-chain)
`.github/workflows/{ci,release,docs}.yml` pin every action to a moving major tag (`actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `softprops/action-gh-release@v2`, `actions/setup-node@v4`, `setup-go@v5`, …). A compromised or retagged action runs in a workflow that holds `secrets.GITHUB_TOKEN` and publishes release binaries.
**Fix:** pin third-party actions to full commit SHAs; keep tags only for first-party `actions/*` if you accept that risk. (Good news: no `pull_request_target` usage was found.)

### M4 — Medium: release workflow ships unsigned, unverifiable binaries
`release.yml` builds and uploads artifacts via `softprops/action-gh-release` with no code signing, checksums, SBOM, or provenance/attestation. Users of a security tool cannot verify what they downloaded. **(Unconfirmed whether any signing happens downstream.)**
**Fix:** publish SHA256SUMS, sign with Sigstore/cosign or minisign, and add build provenance.

### M5 — Medium: `make setup` is incomplete (demo-firebase race, missing deps)
`make demo-firebase` failed first run with `tsx: not found` because the demo was started before `npm install` finished; it works on retry. Combined with the missing `screen`/`pexpect` (C1), `make setup` does not leave a new user in a runnable state.
**Fix:** make `setup` install all Python/Node/system deps and block until done; have the demo targets depend on their install step.

### M6 — Medium: evilginx runs with `-developer -debug` and self-signs silently
The launcher runs evilginx with `-developer -debug`; when upstream cert cloning fails it silently falls back to a self-signed cert (`Evilginx Signature Trust Co.` / 1024-bit key). In a real engagement a silent self-signed fallback changes the victim-facing TLS and can tip off the target or break the capture without a clear operator-facing error.
**Fix:** surface cert failures to the operator instead of silent fallback; make `-debug` opt-in.

### M7 — Medium: captured-session and recipient data have no retention/scrubbing
Captured sessions, credentials, and recipient PII persist in the plaintext DB (H1/H2) with no documented retention, redaction, or secure-delete.
**Fix:** add scoped retention and a "wipe engagement" that securely removes captured secrets.

### L1 — Low: `start-lure` leaves a detached `screen` session and bound :443/:53 after a "failed" start
Because of the false-negative (C1.3), a "failed" launch still leaves evilginx running in `screen phishkit-evilginx`. Operators won't know to `screen -S phishkit-evilginx -X quit`.
**Fix:** on wrapper failure, tear down the screen session; document the session name.

### L2 — Low: colliding/garbage phishlets accumulate
`ensure-destination` adds a new phishlet per destination; collisions (C1.5) are only warned, never cleaned.
**Fix:** dedupe by hostname; offer a prune.

### L3 — Low: error messages point to GUI ("Advanced → Tail evilginx log") from the CLI
CLI JSON errors reference desktop-only UI, unhelpful in headless/CLI use.
**Fix:** give CLI-appropriate remediation (log path, `screen -r`).

### L4 — Low: demo phishlet credential regex `(.*)` captures everything unfiltered
`demo-cookie.yaml` uses `search: '(.*)'` for username/password — fine for a demo, but copied into real phishlets it over-captures.
**Fix:** ship a commented, scoped example.

---

## 4. Documentation errors found
- README (and `make setup`) omit `screen` and `pexpect`, and omit that evilginx needs root/`setcap` for :443/:53 — the core flow cannot run as written (C1).
- README does not mention that the shipped demo phishlet cannot actually proxy the HTTP-only demo mock (C1.4); only the phishlet's inline comment says so.
- `demo-firebase` quick start races `npm install` (M5).
- No documented teardown for the evilginx `screen` session (L1).

## 5. Top improvements (highest leverage first)
1. Make one AiTM demo work end to end: deps in `setup`, privileged-port story, fixed bind-poll, and a phishlet whose upstream is a real reachable HTTPS origin distinct from the lure host (C1). This is the whole product.
2. Protect secrets at rest: `0600` DB/dir + encrypt the SMTP password and captured tokens (H1/H2/M7).
3. Fix recon safety: verify TLS, denylist internal targets (H3).
4. Add a lab-mail path (plaintext/STARTTLS/custom-CA) so engagements can use a local sink (H5).
5. Enforce AUP on every send (H4); patch deps and add `cargo audit` + SHA-pinned, signed releases (M2/M3/M4).

## 6. What I could not fully test / unconfirmed
- The desktop (Tauri) GUI AiTM flow — I exercised the CLI end to end; GUI parity for AUP enforcement (H4) and start-lure is inferred from shared core code, not clicked through.
- A *successful* evilginx capture — blocked by C1; I verified the failure chain (deps, privileges, false-negative, self-proxy loop) but never captured a live session because the shipped phishlet cannot proxy the demo.
- Release-artifact signing downstream of CI (M4) — reviewed workflow only.
- Only local demo targets were used; no third-party site was proxied or attacked, per scope.
