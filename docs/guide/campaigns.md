# Campaign guide

phishkit's native engine owns delivery: you compose a campaign, review it,
send a test, and launch it — with scheduling, delivery-event tracking, and
reporting. There are three ways in, all backed by the same engine.

## Composer (default)

The composer is a draft → review → test → launch flow bound to an assessment,
target, named lure, sender identity, template, and recipient list.

1. **Draft.** Choose the target and named lure, the sender account, the
   template, and the recipient list. Set the mode and (optionally) a schedule
   and rate. Saving creates a draft campaign.
2. **Review.** phishkit runs readiness checks (sender configured, template has
   `{{link}}`, list non-empty, lure reachable, mode consistent) and reports each
   as a pass/fail card so missing dependencies are actionable rather than silent.
3. **Test.** Send a single rendered message to an address you control.
4. **Launch.** Start sending. A launched campaign transitions through
   scheduled → running → paused/completed.

### Snapshots and auditability

At creation, phishkit snapshots the **sender identity** and the **rendered
content**. Later edits to the underlying template or sender do not rewrite a
launched campaign's audit history — the snapshot is what was actually sent.

## Express

Express is the fast path for experienced operators: bind an existing lure link,
sender, template, and list and send, skipping the guided steps. It does **not**
silently auto-create a starter template — missing dependencies are surfaced so
you stay in control.

## Guided

The guided wizard is built for non-technical business users. It walks
target → phishlet/lure → template → recipients → sender → review → launch with
inferred safe defaults and inline "why" text at each step.

It is driven by a curated **preset scenario library** — each preset bundles a
phishlet pattern, an email template, a recipient-CSV shape, and lure defaults —
so a business user can run a safe click-through campaign out of the box, then
hand off to an operator for anything advanced.

## Awareness mode

Awareness mode runs **click-only** training campaigns using a built-in landing
page / redirector. It records who clicked but **never captures credentials**.
Use it when the goal is awareness metrics and education. Credential-capturing
(AiTM) mode is a separate, explicit choice; do not present an AiTM campaign as
awareness training.

## Scheduling and send windows

A campaign can launch immediately or at a scheduled time, restrict sending to a
send window, and throttle to a rate limit. The runner honors these as it moves
the campaign through its lifecycle, and a single campaign runs at a time.

## Delivery events and the funnel

Results shows the full funnel: queued, sent/accepted, delivered, opened,
clicked, bounced, complained, lure visits, and captures.

- Provider **message IDs** are recorded at send time (for ESP APIs that return
  them) so events reconcile to the right attempt.
- **Delivery events** (delivered / bounced / opened / complained) can be
  ingested from your provider. Paste provider event JSON into the Results
  import panel, or pipe it via the [CLI](/guide/cli) `import-events` command.
- Captures are attributed to the originating attempt via the per-attempt
  tracking token, falling back to recipient email.

## Reporting and export

- **Per-campaign report** aggregates the funnel and a per-recipient timeline.
- **Export** as CSV or JSON from Results, or via the CLI
  `export-campaign-report`.
- Exports redact recipient PII and secrets by default; full exports are
  explicit.

## Sessions

Captured sessions have their own focused view: search and filter by status,
recipient, lure, and time; a detail drawer with a timeline, masked credentials,
and a token/cookie summary; an export menu (cookies.txt / JSON / redacted
bundle); and gated, allow-listed incognito replay for operator verification.
Results deep-links straight to the relevant session.

## Assessment lifecycle

When an engagement ends, the assessment overview provides:

- **Export bundle** — a JSON export of the assessment, redacted by default.
- **Selective purge** — delete sessions, attempts, and/or PII while keeping
  reusable phishlets and templates.
- **`/etc/hosts` cleanup** — remove the host entries added for the target.
- **Archive** — mark the assessment inactive (data stays in the database;
  restore or New from archive later). Optionally clean hosts first.
- **Delete** — permanently erase the assessment and engagement-owned data from
  the app database (available after archive). Shared `kit/evilginx/phishlets/` YAML
  is not removed.
