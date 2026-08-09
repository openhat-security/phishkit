# Command line

`phishkit` / `phishkit_ctl` is the headless control plane. It runs the **same
code paths** as the desktop UI (via `crates/phishkit-core`), so the full funnel —
assessment → target → lure → mail → campaign → results → sessions — can be
scripted for end-to-end automation.

## Build and run

```bash
make cli
# or:
cargo run -p phishkit-cli --bin phishkit -- help
./target/release/phishkit help
./target/release/phishkit_ctl help
```

Every command prints JSON to stdout. Errors print JSON on stderr:
`{"error":"…"}`. Flags accept short and long forms (e.g. `-i` / `--id`,
`-p` / `--profile-id`). Payloads accept inline JSON or `@file.json`; CSV/YAML
flags accept inline text or `@file`. Run `phishkit --help` for the full
colored reference (set `NO_COLOR=1` to disable ANSI).

## Setup / paths

```bash
phishkit setup-get
phishkit setup-complete --json '{"setupComplete":true,"persona":"cybersecStudent",…}'
phishkit tutorial-complete --done true
phishkit paths
```

## Recon / proxy

```bash
phishkit kit-info
phishkit service-status
phishkit build
phishkit detect --url <target>
phishkit resolve --target <host> [--dryrun <dom>] [--phishlet <name>]
phishkit ensure-destination --target <host> [--name <profile>] [--force-scaffold] [--assessment <id>]
phishkit scaffold --target <host> --template <pattern-id>
phishkit hosts-status --dryrun <dom> [--phishlet <name>]
phishkit hosts-fix --dryrun <dom> [--phishlet <name>]
phishkit hosts-remove --dryrun <dom> [--phishlet <name>]
phishkit start-lure --profile-id <id> --dryrun <dom> --phishlet <name>
phishkit stop
phishkit list-redirectors
phishkit ca-trust
phishkit open-ca-cert
phishkit tail-logs [--lines <n>]
```

## Profiles / community

```bash
phishkit list-profiles
phishkit get-profile --id <id>
phishkit upsert-profile --json '{…}'
phishkit activate-profile --id <id>
phishkit delete-profile --id <id>
phishkit sync-community
phishkit list-community [--query <q>]
phishkit import-community --name <phishlet>
phishkit list-active-phishlets
phishkit get-phishlet --name <name>
phishkit save-phishlet --name <name> --yaml @file.yaml
phishkit target-readiness --profile-id <id>
```

## Assessments

```bash
phishkit list-assessments [--all]
phishkit get-assessment --id <id>
phishkit create-assessment --json '{"name":"…","primaryDomain":"…"}'
phishkit update-assessment --json '{"id":"…",…}'
phishkit set-active-assessment --id <id>
phishkit get-active-assessment
phishkit archive-assessment --id <id>
phishkit unarchive-assessment --id <id>
phishkit clone-assessment --id <id>
phishkit delete-assessment --id <id>
phishkit list-targets --assessment <id>
phishkit export-assessment --id <id> [--no-redact]
phishkit purge-assessment --id <id> [--sessions] [--attempts] [--pii]
phishkit assessment-hosts-cleanup --id <id>
```

Export is redacted by default; `--no-redact` produces a full bundle.

## Lures

```bash
phishkit list-lures --profile-id <id>
phishkit get-lure --id <id>
phishkit get-default-lure --profile-id <id>
phishkit upsert-lure --json '{"profileId":"…","name":"…"}'
phishkit set-default-lure --profile-id <id> --lure-id <id>
phishkit delete-lure --id <id>
```

## Mail and content

```bash
phishkit list-mail-accounts
phishkit upsert-mail-account --json '{…}'
phishkit activate-mail-account --id <id>
phishkit delete-mail-account --id <id>
phishkit send-test --to <email>
phishkit list-templates [--assessment <id>]
phishkit upsert-template --json '{"name":"…","subject":"…","htmlBody":"…"}'
phishkit delete-template --id <id>
phishkit list-recipient-lists [--assessment <id>]
phishkit create-list --name <name> [--assessment <id>]
phishkit delete-list --id <id>
phishkit import-recipients --list-id <id> --csv @recipients.csv
phishkit list-recipients --list-id <id>
```

## Campaigns and results

```bash
phishkit list-campaigns [--assessment <id>]
phishkit get-campaign --id <id>
phishkit create-campaign --json '{"name":"…","templateId":"…","listId":"…","linkUrl":"…"}'
phishkit delete-campaign --id <id>
phishkit campaign-review --id <id>
phishkit send-campaign-test --id <id> --to <email>
phishkit start-campaign --id <id>
phishkit stop-campaign --id <id>
phishkit retry-failed --id <id>
phishkit campaign-attempts --id <id>
phishkit campaign-funnel --id <id>
phishkit campaign-report --id <id>
phishkit export-campaign-report --id <id> [--format csv|json]
phishkit import-events --id <id> --raw @events.json
```

## Sessions

```bash
phishkit sync-captures --profile-id <id>
phishkit list-captures --profile-id <id>
phishkit delete-capture --profile-id <id> --session-id <n>
phishkit prune-captures --profile-id <id>
phishkit export-cookies --profile-id <id> --session-id <n> [--format json|netscape]
phishkit attribute-captures --profile-id <id>
phishkit launch-replay --profile-id <id> --session-id <n> --api-key <key>
```

## AUP

```bash
phishkit aup-status
phishkit aup-accept
```

## Wizards (interactive)

TTY-only guided flows for **authorized** engagements (`wizard` is an alias):

```bash
phishkit wiz                 # menu
phishkit wiz quickstart      # new assessment → destination → SMTP → next steps
phishkit wiz send            # template / list / campaign (SMTP already set up)
phishkit wiz sessions        # sync / browse / export captures
```

Each wizard requires an affirmative authorized-use confirmation before proxy or
mail steps. See [authorized use](/guide/authorized-use).
