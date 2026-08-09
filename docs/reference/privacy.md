# Privacy

phishkit runs locally and is designed to keep sensitive data on the operator's
machine. It does not include analytics or telemetry and does not phone home.

## What phishkit stores

- **Application database** — assessments, targets, lures, templates, recipient
  lists, campaigns, campaign snapshots, per-recipient attempts, and synced
  captures. Stored as `phishkit.db` under the OS application data directory
  (or a custom / ephemeral path from Setup). A legacy checkout
  `run/**/phishkit.db` is migrated forward on first run, leaving a `.bak`.
- **evilginx capture database** — credentials and session tokens captured by the
  proxy, under the OS data dir `evilginx/` (legacy fallback:
  `kit/evilginx/run/data/`).
- **Sender credentials** — SMTP/ESP settings you configure, in the app database
  (secrets are masked in API/CLI responses).
- **Exports** — any bundles, reports, cookies.txt/JSON, or redacted exports you
  choose to write.

Use `phishkit paths` (or Settings) to see the resolved directories on your machine.
Mutable state is gitignored and excluded from source control.

## What phishkit sends

phishkit only makes the network requests an assessment requires. See
[local data and network activity](/reference/data-and-network) for the full
inventory. In short:

- Email to **your** SMTP relay or ESP API.
- Proxy traffic between the assessed user and the real target application
  (evilginx).
- Optional community phishlet-pack downloads when you run that sync.

There is no product analytics, crash reporting, or usage telemetry.

## Recipient and subject data

Recipient lists contain real people's data. phishkit:

- redacts recipient PII, credentials, and tokens in exports and reports by
  default (full exports are explicit);
- masks credentials and tokens in the Sessions detail view;
- expects you to operate only under written authorization and your
  organization's retention rules.

Delete assessments and purge engagement data when an engagement ends. OS data
directories are not wiped by uninstalling the git checkout alone.
