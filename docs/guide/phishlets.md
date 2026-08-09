# Phishlet authoring

A **phishlet** is evilginx's YAML description of how to proxy a specific login
flow: which hosts to proxy, which tokens and credentials to capture, and how to
rewrite content. phishkit ships starter templates and can scaffold a phishlet
for a target, but every engagement needs its phishlet reviewed and customized.

## Where phishlets live

- `evilginx/phishlet-templates/` — reusable scaffolding templates, including
  `cookie-sso`, `firebase`, `generic-spa`, `jwt-api`, and `oauth-oidc`.
- `kit/evilginx/phishlets/` — per-target phishlets for your assessments, including
  ready demos `demo-cookie` (cookie session) and `demo-firebase` (token/replay).
- `kit/evilginx/phishlets/generic.yaml` — blank engagement template (copy and
  customize; do not treat demos as client kits).
- `vendor/community-phishlets/` — vendored third-party packs. Import in the desktop
  app under Target → Recon & Proxy → **Community** (see `demos/community/`).
- `demos/` — Target notes for demos and an annotated community shortlist.

## Run demos first

```bash
make demo-cookie                              # http://127.0.0.1:9080
make validate-phishlet PHISHLET=demo-cookie
make demo-firebase                            # http://127.0.0.1:9081
make validate-phishlet PHISHLET=demo-firebase
```

Creds: `demo@phishkit.local` / `demo-password`. Full notes: `demos/cookie/`,
`demos/firebase/`.

## Scaffold from a target

phishkit can inspect a target and scaffold a phishlet from the closest template:

- In the app, add a Target and use **generate/import** to create its
  destination and phishlet.
- On the command line:

```bash
# detect the login flow
phishkit_ctl detect --url https://app.example.com

# scaffold a phishlet from a template pattern
phishkit_ctl scaffold --target app.example.com --template oauth-oidc

# or do detect + scaffold + profile in one step
phishkit_ctl ensure-destination --target app.example.com --assessment <id>
```

See the [CLI guide](/guide/cli) for the full command set.

## Customize per engagement

Open the scaffolded phishlet and fill in, at minimum:

- **`proxy_hosts`** — the real hostnames to proxy and how to rewrite them.
- **`auth_tokens`** — the cookies/headers/body tokens that constitute a valid
  session (this is what makes a capture useful).
- **`credentials`** — where the username/password land in the request.
- **`sub_filters`** — content rewrites so the proxied pages work and point back
  through the lure.
- **`js_inject`** — optional injected JavaScript for flows that need it.

The template files contain extensive comments explaining each section.

## Validate

```bash
make validate-phishlet PHISHLET=<name>
```

## Lures

A **lure** is a tracked link into the proxy for a phishlet. phishkit uses named
lures per target, and each attempt carries a tracking token so captures attribute
deterministically to the campaign attempt. Manage lures in the app or via the
CLI (`list-lures`, `upsert-lure`, `set-default-lure`).

## Test locally

Start the proxy for a local dry-run, apply `/etc/hosts`, then open the lure URL
in your own browser to confirm the flow proxies and a capture is produced before
you attach it to a campaign. See the [quick start](/guide/quick-start).
