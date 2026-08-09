//! Colored, structured CLI help for `phishkit` / `phishkit_ctl`.

use std::fmt::Write as _;
use std::io::IsTerminal;

/// True when stderr should use ANSI colors (TTY and `NO_COLOR` unset).
pub fn want_color() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if std::env::var_os("FORCE_COLOR").is_some() {
        return true;
    }
    std::io::stderr().is_terminal()
}

struct Style {
    on: bool,
}

impl Style {
    fn new(on: bool) -> Self {
        Self { on }
    }
    fn paint(&self, code: &str, s: &str) -> String {
        if self.on {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }
    fn bold(&self, s: &str) -> String {
        self.paint("1", s)
    }
    fn dim(&self, s: &str) -> String {
        self.paint("2", s)
    }
    fn cyan(&self, s: &str) -> String {
        self.paint("1;36", s)
    }
    fn green(&self, s: &str) -> String {
        self.paint("1;32", s)
    }
    fn yellow(&self, s: &str) -> String {
        self.paint("1;33", s)
    }
    fn magenta(&self, s: &str) -> String {
        self.paint("1;35", s)
    }
}

struct Cmd {
    usage: &'static str,
    summary: &'static str,
}

struct Section {
    title: &'static str,
    blurb: &'static str,
    cmds: &'static [Cmd],
}

const SECTIONS: &[Section] = &[
    Section {
        title: "Setup & paths",
        blurb: "First-run config, tutorial flag, and where data lives on disk.",
        cmds: &[
            Cmd {
                usage: "paths",
                summary: "Print resolved config, data, DB, evilginx, and kit paths",
            },
            Cmd {
                usage: "setup-get",
                summary: "Show current setup.json (persona, storage mode, flags)",
            },
            Cmd {
                usage: "setup-complete  -j, --json <obj|@file>",
                summary: "Mark setup complete and persist preferences",
            },
            Cmd {
                usage: "tutorial-complete  -D, --done true|false",
                summary: "Set whether the in-app tutorial is finished",
            },
        ],
    },
    Section {
        title: "Recon & proxy",
        blurb: "Detect targets, scaffold phishlets, hosts, and run evilginx.",
        cmds: &[
            Cmd {
                usage: "kit-info",
                summary: "Kit root, binary presence, and version-ish metadata",
            },
            Cmd {
                usage: "service-status",
                summary: "Whether evilginx is running (and pid when known)",
            },
            Cmd {
                usage: "build",
                summary: "Build kit binaries (make build / evilginx)",
            },
            Cmd {
                usage: "detect  -u, --url <target>",
                summary: "Fingerprint a login URL / host (stack hints)",
            },
            Cmd {
                usage: "resolve  -t, --target <host>  [-d, --dryrun <dom>]  [-P, --phishlet <name>]",
                summary: "Compute dry-run domain + phishlet name for a target",
            },
            Cmd {
                usage: "ensure-destination  -t, --target <host>  [-n, --name <profile>]  [-F, --force-scaffold]  [-a, --assessment <id>]",
                summary: "Detect → scaffold → upsert target profile in one step",
            },
            Cmd {
                usage: "scaffold  -t, --target <host>  -T, --template <pattern-id>",
                summary: "Generate a phishlet YAML from a pattern template",
            },
            Cmd {
                usage: "hosts-status  -d, --dryrun <dom>  [-P, --phishlet <name>]",
                summary: "Check /etc/hosts entries for the dry-run domain",
            },
            Cmd {
                usage: "hosts-fix  -d, --dryrun <dom>  [-P, --phishlet <name>]",
                summary: "Add required dry-run hosts (may prompt for admin)",
            },
            Cmd {
                usage: "hosts-remove  -d, --dryrun <dom>  [-P, --phishlet <name>]",
                summary: "Remove dry-run hosts entries",
            },
            Cmd {
                usage: "start-lure  -p, --profile-id <id>  -d, --dryrun <dom>  -P, --phishlet <name>",
                summary: "Configure and start evilginx; print lure URL",
            },
            Cmd {
                usage: "stop",
                summary: "Stop the background evilginx session",
            },
            Cmd {
                usage: "list-redirectors",
                summary: "List available HTML redirector templates",
            },
            Cmd {
                usage: "ca-trust",
                summary: "Local CA trust status for developer certs",
            },
            Cmd {
                usage: "open-ca-cert",
                summary: "Path (and open) to the evilginx developer CA cert",
            },
            Cmd {
                usage: "tail-logs  [-l, --lines <n>]",
                summary: "Tail the evilginx log (default ~80 lines)",
            },
        ],
    },
    Section {
        title: "Profiles & community",
        blurb: "Target profiles and third-party phishlet packs.",
        cmds: &[
            Cmd {
                usage: "list-profiles",
                summary: "List all target profiles",
            },
            Cmd {
                usage: "get-profile  -i, --id <id>",
                summary: "Fetch one profile by id",
            },
            Cmd {
                usage: "upsert-profile  -j, --json <obj|@file>",
                summary: "Create or update a profile",
            },
            Cmd {
                usage: "activate-profile  -i, --id <id>",
                summary: "Set the active profile for the UI/runtime",
            },
            Cmd {
                usage: "delete-profile  -i, --id <id>",
                summary: "Delete a profile and its owned rows",
            },
            Cmd {
                usage: "sync-community",
                summary: "Refresh vendor/community-phishlets from lockfile pins",
            },
            Cmd {
                usage: "list-community  [-q, --query <text>]",
                summary: "List / search community phishlet catalog",
            },
            Cmd {
                usage: "import-community  -n, --name <phishlet>",
                summary: "Copy a community YAML into kit/evilginx/phishlets",
            },
            Cmd {
                usage: "list-active-phishlets",
                summary: "Phishlets present under kit/evilginx/phishlets",
            },
            Cmd {
                usage: "get-phishlet  -n, --name <name>",
                summary: "Read a phishlet YAML from the kit",
            },
            Cmd {
                usage: "save-phishlet  -n, --name <name>  -y, --yaml <text|@file>",
                summary: "Write / overwrite a kit phishlet YAML",
            },
            Cmd {
                usage: "target-readiness  -p, --profile-id <id>",
                summary: "Checklist: phishlet, hosts, lure, proxy readiness",
            },
        ],
    },
    Section {
        title: "Assessments",
        blurb: "Engagement containers: scope, archive, export, purge.",
        cmds: &[
            Cmd {
                usage: "list-assessments  [-A, --all]",
                summary: "List active assessments (include archived with --all)",
            },
            Cmd {
                usage: "get-assessment  -i, --id <id>",
                summary: "Fetch one assessment",
            },
            Cmd {
                usage: "create-assessment  -j, --json {\"name\",\"primaryDomain\",…}",
                summary: "Create an assessment + primary scope",
            },
            Cmd {
                usage: "update-assessment  -j, --json {\"id\",…}",
                summary: "Patch assessment fields",
            },
            Cmd {
                usage: "set-active-assessment  -i, --id <id>",
                summary: "Set the active assessment context",
            },
            Cmd {
                usage: "get-active-assessment",
                summary: "Show the active assessment (if any)",
            },
            Cmd {
                usage: "archive-assessment  -i, --id <id>",
                summary: "Soft-archive (status=archived)",
            },
            Cmd {
                usage: "unarchive-assessment  -i, --id <id>",
                summary: "Restore an archived assessment",
            },
            Cmd {
                usage: "delete-assessment  -i, --id <id>",
                summary: "Hard-delete assessment and owned DB rows",
            },
            Cmd {
                usage: "clone-assessment  -i, --id <id>",
                summary: "Clone assessment into a new active engagement",
            },
            Cmd {
                usage: "list-targets  -a, --assessment <id>",
                summary: "List profiles (targets) for an assessment",
            },
            Cmd {
                usage: "export-assessment  -i, --id <id>  [-N, --no-redact]",
                summary: "Export bundle JSON (redacted by default)",
            },
            Cmd {
                usage: "purge-assessment  -i, --id <id>  [--sessions] [--attempts] [--pii]",
                summary: "Selective data wipe inside an assessment",
            },
            Cmd {
                usage: "assessment-hosts-cleanup  -i, --id <id>",
                summary: "Remove hosts entries for assessment dry-runs",
            },
        ],
    },
    Section {
        title: "Lures",
        blurb: "Per-profile lure paths, OG tags, and defaults.",
        cmds: &[
            Cmd {
                usage: "list-lures  -p, --profile-id <id>",
                summary: "List lures for a profile",
            },
            Cmd {
                usage: "get-lure  -i, --id <id>",
                summary: "Fetch one lure",
            },
            Cmd {
                usage: "get-default-lure  -p, --profile-id <id>",
                summary: "Default lure for a profile",
            },
            Cmd {
                usage: "upsert-lure  -j, --json {\"profileId\",\"name\",…}",
                summary: "Create or update a lure",
            },
            Cmd {
                usage: "set-default-lure  -p, --profile-id <id>  -U, --lure-id <id>",
                summary: "Mark a lure as the profile default",
            },
            Cmd {
                usage: "delete-lure  -i, --id <id>",
                summary: "Delete a lure",
            },
        ],
    },
    Section {
        title: "Mail & content",
        blurb: "Senders, templates, and recipient lists.",
        cmds: &[
            Cmd {
                usage: "list-mail-accounts",
                summary: "List SMTP/ESP accounts (secrets masked)",
            },
            Cmd {
                usage: "upsert-mail-account  -j, --json <obj|@file>",
                summary: "Create or update a mail account",
            },
            Cmd {
                usage: "activate-mail-account  -i, --id <id>",
                summary: "Set the active sender",
            },
            Cmd {
                usage: "delete-mail-account  -i, --id <id>",
                summary: "Delete a mail account",
            },
            Cmd {
                usage: "send-test  -e, --to <email>",
                summary: "Send a test message via the active account",
            },
            Cmd {
                usage: "list-templates  [-a, --assessment <id>]",
                summary: "List email templates (optionally scoped)",
            },
            Cmd {
                usage: "upsert-template  -j, --json {\"name\",\"subject\",\"htmlBody\",…}",
                summary: "Create or update a template",
            },
            Cmd {
                usage: "delete-template  -i, --id <id>",
                summary: "Delete a template",
            },
            Cmd {
                usage: "list-recipient-lists  [-a, --assessment <id>]",
                summary: "List recipient lists",
            },
            Cmd {
                usage: "create-list  -n, --name <name>  [-a, --assessment <id>]",
                summary: "Create an empty recipient list",
            },
            Cmd {
                usage: "delete-list  -i, --id <id>",
                summary: "Delete a recipient list",
            },
            Cmd {
                usage: "import-recipients  -L, --list-id <id>  -c, --csv <text|@file>",
                summary: "Import CSV rows into a list",
            },
            Cmd {
                usage: "list-recipients  -L, --list-id <id>",
                summary: "List recipients in a list",
            },
        ],
    },
    Section {
        title: "Campaigns & results",
        blurb: "Launch mail campaigns and inspect funnel / reports.",
        cmds: &[
            Cmd {
                usage: "list-campaigns  [-a, --assessment <id>]",
                summary: "List campaigns",
            },
            Cmd {
                usage: "get-campaign  -i, --id <id>",
                summary: "Fetch one campaign",
            },
            Cmd {
                usage: "create-campaign  -j, --json {\"name\",\"templateId\",\"listId\",\"linkUrl\",…}",
                summary: "Create a campaign",
            },
            Cmd {
                usage: "delete-campaign  -i, --id <id>",
                summary: "Delete a campaign",
            },
            Cmd {
                usage: "campaign-review  -i, --id <id>",
                summary: "Pre-send review checklist",
            },
            Cmd {
                usage: "send-campaign-test  -i, --id <id>  -e, --to <email>",
                summary: "Send a one-off test for a campaign",
            },
            Cmd {
                usage: "start-campaign  -i, --id <id>",
                summary: "Start / resume sending",
            },
            Cmd {
                usage: "stop-campaign  -i, --id <id>",
                summary: "Stop a running campaign",
            },
            Cmd {
                usage: "retry-failed  -i, --id <id>",
                summary: "Retry failed delivery attempts",
            },
            Cmd {
                usage: "campaign-attempts  -i, --id <id>",
                summary: "Per-recipient attempt rows",
            },
            Cmd {
                usage: "campaign-funnel  -i, --id <id>",
                summary: "Funnel counts (sent → open → click → …)",
            },
            Cmd {
                usage: "campaign-report  -i, --id <id>",
                summary: "Structured campaign report JSON",
            },
            Cmd {
                usage: "export-campaign-report  -i, --id <id>  [-f, --format csv|json]",
                summary: "Export report as CSV or JSON text",
            },
            Cmd {
                usage: "import-events  -i, --id <id>  -r, --raw <text|@file>",
                summary: "Import ESP delivery events JSON",
            },
        ],
    },
    Section {
        title: "Sessions",
        blurb: "Sync evilginx captures, export cookies, optional replay.",
        cmds: &[
            Cmd {
                usage: "sync-captures  -p, --profile-id <id>",
                summary: "Pull evilginx sessions into the app DB",
            },
            Cmd {
                usage: "list-captures  -p, --profile-id <id>",
                summary: "List synced captures for a profile",
            },
            Cmd {
                usage: "delete-capture  -p, --profile-id <id>  -s, --session-id <n>",
                summary: "Ignore + delete one capture",
            },
            Cmd {
                usage: "prune-captures  -p, --profile-id <id>",
                summary: "Drop empty / useless capture rows",
            },
            Cmd {
                usage: "export-cookies  -p, --profile-id <id>  -s, --session-id <n>  [-f, --format json|netscape]",
                summary: "Export session tokens as cookies",
            },
            Cmd {
                usage: "attribute-captures  -p, --profile-id <id>",
                summary: "Match captures to campaign sends",
            },
            Cmd {
                usage: "launch-replay  -p, --profile-id <id>  -s, --session-id <n>  -k, --api-key <key>",
                summary: "Open browser + restore script for a capture",
            },
        ],
    },
    Section {
        title: "AUP",
        blurb: "Acceptable use acknowledgement.",
        cmds: &[
            Cmd {
                usage: "aup-status",
                summary: "Whether AUP has been accepted locally",
            },
            Cmd {
                usage: "aup-accept",
                summary: "Record AUP acceptance",
            },
        ],
    },
    Section {
        title: "Wizards",
        blurb: "Interactive TTY flows for authorized engagements (alias: wizard).",
        cmds: &[
            Cmd {
                usage: "wiz",
                summary: "Show wizard menu",
            },
            Cmd {
                usage: "wiz quickstart",
                summary: "New assessment end-to-end: target → hosts → SMTP → next steps",
            },
            Cmd {
                usage: "wiz send",
                summary: "Pick template/list, preview, create campaign / test send",
            },
            Cmd {
                usage: "wiz sessions",
                summary: "Sync captures, browse sessions, export cookies",
            },
        ],
    },
];

/// Render full help. When `color` is false, emit plain text (tests / pipes).
pub fn render_help(color: bool) -> String {
    let s = Style::new(color);
    let mut out = String::new();

    let _ = writeln!(
        out,
        "{}  {}",
        s.cyan("phishkit"),
        s.dim("· headless control plane  (alias: phishkit_ctl)")
    );
    let _ = writeln!(
        out,
        "{}",
        s.dim("Same engine as the desktop app · JSON on stdout · errors as {\"error\":\"…\"} on stderr")
    );
    let _ = writeln!(out);

    let _ = writeln!(out, "{}", s.bold(&s.magenta("USAGE")));
    let _ = writeln!(
        out,
        "  {} {} {}",
        s.green("phishkit"),
        s.yellow("<command>"),
        s.dim("[options]")
    );
    let _ = writeln!(
        out,
        "  {} {}",
        s.green("phishkit help"),
        s.dim("               show this page")
    );
    let _ = writeln!(out);

    let _ = writeln!(out, "{}", s.bold(&s.magenta("GLOBAL")));
    let _ = writeln!(
        out,
        "  {}  {}",
        s.yellow("-h, --help"),
        s.dim("Show help and exit")
    );
    let _ = writeln!(
        out,
        "  {}   {}",
        s.yellow("NO_COLOR=1"),
        s.dim("Disable ANSI colors")
    );
    let _ = writeln!(out);

    let _ = writeln!(out, "{}", s.bold(&s.magenta("COMMON FLAGS")));
    for (flags, desc) in [
        ("-i, --id <id>", "Resource id"),
        ("-p, --profile-id <id>", "Target profile id"),
        ("-a, --assessment <id>", "Assessment scope"),
        ("-t, --target <host>", "Target hostname / URL host"),
        ("-u, --url <url>", "Full URL for detect"),
        ("-d, --dryrun <dom>", "Dry-run / lookalike domain"),
        ("-P, --phishlet <name>", "Phishlet basename"),
        ("-n, --name <name>", "Display name"),
        ("-j, --json <obj|@file>", "JSON body (or @path to file)"),
        ("-e, --to <email>", "Recipient email"),
        ("-L, --list-id <id>", "Recipient list id"),
        ("-s, --session-id <n>", "evilginx session id"),
        ("-f, --format <fmt>", "Export format"),
        ("-c, --csv <text|@file>", "CSV payload"),
        ("-y, --yaml <text|@file>", "YAML payload"),
        ("-r, --raw <text|@file>", "Raw JSON/events payload"),
        ("-k, --api-key <key>", "API key (replay)"),
        ("-A, --all", "Include archived / full lists"),
        ("-N, --no-redact", "Full (unredacted) export"),
        ("-F, --force-scaffold", "Overwrite scaffolded phishlet"),
    ] {
        let _ = writeln!(out, "  {:<28} {}", s.yellow(flags), s.dim(desc));
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{}",
        s.dim("  Tip: prefix a value with @ to read it from a file  (e.g. -j @req.json)")
    );
    let _ = writeln!(out);

    for section in SECTIONS {
        let _ = writeln!(out, "{}", s.bold(&s.magenta(section.title)));
        let _ = writeln!(out, "  {}", s.dim(section.blurb));
        let _ = writeln!(out);
        for cmd in section.cmds {
            // Split command name from flags for coloring
            let (name, rest) = match cmd.usage.split_once("  ") {
                Some((n, r)) => (n, r),
                None => (cmd.usage, ""),
            };
            if rest.is_empty() {
                let _ = writeln!(out, "  {}", s.green(name));
            } else {
                let _ = writeln!(out, "  {}  {}", s.green(name), s.yellow(rest));
            }
            let _ = writeln!(out, "      {}", s.dim(cmd.summary));
        }
        let _ = writeln!(out);
    }

    let _ = writeln!(out, "{}", s.bold(&s.magenta("EXAMPLES")));
    for line in [
        "phishkit paths",
        "phishkit detect -u https://app.example.com/login",
        "phishkit create-assessment -j '{\"name\":\"Q2\",\"primaryDomain\":\"example.com\"}'",
        "phishkit list-assessments -A",
        "phishkit start-lure -p my-target -d example.phishkit -P app-example-com",
        "phishkit export-assessment -i <id>          # redacted",
        "phishkit export-assessment -i <id> -N       # full",
    ] {
        let _ = writeln!(out, "  {}", s.dim(line));
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{}",
        s.dim("Docs: docs/guide/cli.md  ·  Build: make cli  ·  cargo run -p phishkit-cli -- help")
    );

    out
}

/// Plain-text help (no ANSI). Used by tests and `help` JSON responses.
pub fn help_plain() -> String {
    render_help(false)
}
