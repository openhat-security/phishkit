# Architecture

phishkit is a single desktop application (Tauri + React front end, Rust back
end) that orchestrates evilginx and a native email engine into one authorized
end-to-end workflow. Everything runs locally on the operator's machine.

## End-to-end flow

```
Assessments Home
      │
      ▼
  Assessment ─────────────► Templates + Recipients
      │                              │
      ▼                              │
   Target (profile)                  │
      ├── evilginx AiTM + preflight  │
      └── Named lures ───────────────┤
                                     ▼
                              Campaign composer
                                     │
             Global senders (SMTP / ESP) ──┤
                                     ▼
                          Native send engine
                          ├── delivery/open/click event ingestion
                          ▼
   evilginx session capture ─────► Results funnel
                                     │
                                     ▼
                     Session detail + attribution
                     + export + gated replay
```

## Components

### Front end (`apps/desktop/src/`)

React app for the assessment workspace: Assessments home and overview,
Destinations (targets/proxy/lures/captures), Templates, Recipients, the Campaign
composer (with Express and Guided flows), Results, and the focused Sessions view.
Guided flows are driven by a preset scenario library (`apps/desktop/src/lib/presets.js`).

### Engine (`crates/phishkit-core/`) + thin Tauri shell (`apps/desktop/src-tauri/`)

Rust owns supported behavior, the database schema, and migrations. Key modules:

- `assessment.rs` — assessment lifecycle: export bundle, selective purge,
  `/etc/hosts` cleanup.
- `campaign.rs` — the campaign engine: composer, snapshots, scheduling/send
  windows, the send loop, delivery-event ingestion, reporting/export, and
  deterministic capture attribution.
- `mail.rs` + `providers.rs` — sending via SMTP or ESP HTTP APIs, and
  provider message-ID extraction.
- `evilginx_ctl.rs`, `phishlet.rs`, `recon.rs`, `destination.rs`, `hosts.rs` —
  the AiTM proxy control plane and host management.
- `sessions.rs`, `firebase.rs` — capture sync/list/export and gated replay.
- `db.rs` — SQLite schema and access.
- `cli.rs` + `apps/cli` — the headless CLI that mirrors the UI.

### AiTM proxy

evilginx (vendored under `vendor/evilginx2`, built to a local binary) proxies the
real login flow for a target and captures credentials and session tokens.

## Data model

An **assessment** contains targets (profiles), named lures, templates, recipient
lists, campaigns, and captured sessions. A **campaign** stores a sender/content
**snapshot** taken at creation; **campaign attempts** track per-recipient state
(queued → sent → delivered/opened/clicked/bounced/complained) plus a tracking
token used to attribute a **capture** back to the originating attempt.

See [local data and network activity](/reference/data-and-network) for where
this is stored and what leaves the machine.

## Local demos and community packs

- `demos/` — TypeScript practice apps (cookie-session demo on :9080, Firebase-shaped on
  :9081) for first-run capture practice. Matching phishlets live under
  `kit/evilginx/phishlets/demo-*.yaml`; copy-ready notes are in `demos/`.
- `vendor/community-phishlets/` — pinned third-party YAML packs shipped in-repo.
  Desktop imports from this path; `make community-phishlets` refreshes pins.
