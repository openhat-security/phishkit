# Community phishlet shortlist (annotated)

**Authorized targets only.** These files are third-party YAML vendored under
`vendor/community-phishlets/`. They are for learning patterns (cookie SSO,
OAuth/OIDC, office-suite style flows) — not a menu of live brands to attack.
Upstream credits and links: [`vendor/community-phishlets/README.md`](../../vendor/community-phishlets/README.md).
Do not enable them in the default dry-run path against production sites.

Desktop lists the full catalog from `vendor/community-phishlets/index.json`.
`make community-phishlets` refreshes pins; packs already ship in-repo.

## Annotated filenames

| Path | Pattern notes | Caution |
|------|---------------|---------|
| `klezvirus/okta.yaml` | Cookie / IdP-style enterprise SSO shape | Real Okta tenants are out of scope unless contracted |
| `klezvirus/o365.yaml` | Microsoft 365–style proxy host + cookie capture | Office 365 production = authorized engagement only |
| `simplerhacking/Okta.yaml` | Alternate Okta community variant; compare `auth_tokens` | Diff before import; unreviewed |
| `simplerhacking/Microsoft2024.yaml` | Newer Microsoft portal host layout | Hostnames drift; re-recon always |
| `simplerhacking/microsoft-o365-adfs.yaml` | ADFS / federated redirect flavor | Watch redirector and landing hosts |
| `anonud4y/o365.yaml` | Community O365 cookie pack | Naming/noise varies by pack |
| `klezvirus/onelogin.yaml` | Another IdP cookie-session example | Same rules as Okta |
| `klezvirus/google.yaml` | Broad consumer Google surface | High HSTS / risk; demos preferred for practice |

## How to import

1. `make desktop` — community packs are on disk; no sync bootstrap required.
2. Open a Target → **Recon & Proxy** → **Community** → search by name → **Import**.
   (Advanced → “Open Community →” jumps to the same browser.)
3. Customize `proxy_hosts` / `auth_tokens` for **your** authorized target.
4. `make validate-phishlet PHISHLET=<imported-name>`
5. Prefer `demos/cookie` / `demos/firebase` for first-run practice.
   New installs can also use the in-app **Demo tour** (sidebar or Assessments).

## Refresh pins

```bash
make community-phishlets   # re-fetch lockfile pins into vendor/community-phishlets/
```
