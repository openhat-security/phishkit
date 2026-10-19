# Demo: portal (cookie session)

## Target notes

| Field | Value |
|-------|-------|
| HTTP mock (no AiTM) | http://127.0.0.1:9080 (`make demo-cookie`) |
| HTTPS origin (AiTM) | https://127.0.0.2:443 (`make demo-cookie-https`) |
| Upstream hostname | `cookie-origin.local.phishkit` → 127.0.0.2 |
| Lure hostname | `demo-cookie.local.phishkit` → 127.0.0.1 |
| Phishlet | `demo-cookie` |
| Login path | `/login` |
| Test user | `demo@phishkit.local` |
| Test password | `demo-password` |

## /etc/hosts

```
127.0.0.1 demo-cookie.local.phishkit
127.0.0.2 cookie-origin.local.phishkit
```

## Privileged ports

evilginx binds `:443` and `:53`. The HTTPS mock binds `127.0.0.2:443` so it does not collide with evilginx on `127.0.0.1`.

```bash
sudo setcap cap_net_bind_service=+ep kit/evilginx/run/evilginx
sudo setcap cap_net_bind_service=+ep "$(command -v node)"
```

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

Authorized dry-runs only.
