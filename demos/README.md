# demos/

Intentionally simple TypeScript practice apps for first-run AiTM dry-runs.
**Authorized lab use only** — see [authorized use](../docs/guide/authorized-use.md).

| App | Port | Make target | Phishlet |
|-----|------|-------------|----------|
| [`cookie/`](cookie/) | `:9080` | `make demo-cookie` | `demo-cookie` |
| [`firebase/`](firebase/) | `:9081` | `make demo-firebase` | `demo-firebase` |

Creds: `demo@phishkit.local` / `demo-password`.

Public tunnel (authorized demos only):

```bash
make demo-tunnel PORT=9080
```

Community pack notes: [`community/README.md`](community/README.md).
