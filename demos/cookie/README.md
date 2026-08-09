# Demo: portal (cookie session) (cookie session)

## Target notes

| Field | Value |
|-------|-------|
| Upstream mock | http://127.0.0.1:9080 |
| Start | `make demo-cookie` |
| Phishlet | `demo-cookie` (`kit/evilginx/phishlets/demo-cookie.yaml`) |
| Dry-run domain | `demo-cookie.local.phishkit` |
| Login path | `/login` |
| Test user | `demo@phishkit.local` |
| Test password | `demo-password` |

## Expected capture

- **Credentials:** form POST fields `username` + `password`
- **Session material:** `Set-Cookie: session=…` (HttpOnly)
- **Success URL:** `/dashboard`

## Smoke test (no evilginx)

```bash
make demo-cookie &
sleep 0.5
curl -s -c /tmp/pk-demo-cookie.jar -b /tmp/pk-demo-cookie.jar -L \
  -X POST http://127.0.0.1:9080/login \
  -d 'username=demo@phishkit.local&password=demo-password' \
  -o /dev/null -w '%{http_code} %{url_effective}\n'
grep session /tmp/pk-demo-cookie.jar
```

## Desktop Target

1. `make desktop`
2. Create an assessment → add Target `demo-cookie.local.phishkit`
3. Import / select phishlet `demo-cookie`
4. Start destination with dry-run domain `demo-cookie.local.phishkit`
5. Apply `/etc/hosts` when prompted; open the lure in a fresh browser profile

Authorized dry-runs only.
