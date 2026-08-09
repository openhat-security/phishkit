# Demo: firebase (token / replay) (token / replay)

## Target notes

| Field | Value |
|-------|-------|
| Upstream mock | http://127.0.0.1:9081 |
| Start | `make demo-firebase` |
| Phishlet | `demo-firebase` (`kit/evilginx/phishlets/demo-firebase.yaml`) |
| Dry-run domain | `demo-firebase.local.phishkit` |
| Login path | `/login` |
| Test user | `demo@phishkit.local` |
| Test password | `demo-password` |
| Fake API key | `AIzaSyDemoPhishkitFirebaseKey0000001` |

## Expected capture

- **Credentials:** JSON `email` + `password` on Identity Toolkit sign-in
- **Tokens:** `idToken` / `refreshToken` (also flat `id_token` / `refresh_token`)
- **Client storage:** `localStorage` key `firebase:authUser:<apiKey>:[DEFAULT]`
- **js_inject:** exfils tokens to `/__evilginx_creds` for evilginx `credentials.custom`

## Replay checklist

1. Sign in at http://127.0.0.1:9081/login and confirm tokens appear on the dashboard.
2. Capture via the `demo-firebase` phishlet (or paste tokens into a session for restore drills).
3. In desktop **Sessions**, confirm `id_token` + `refresh_token` are present.
4. Run gated Firebase restore/replay only against this mock (or another authorized target).
5. Sign out of the mock and verify restore re-hydrates `firebaseLocalStorage` / localStorage.

## Smoke test (no evilginx)

```bash
make demo-firebase &
sleep 0.5
curl -s -X POST \
  'http://127.0.0.1:9081/identitytoolkit.googleapis.com/v1/accounts:signInWithPassword?key=AIzaSyDemoPhishkitFirebaseKey0000001' \
  -H 'Content-Type: application/json' \
  -d '{"email":"demo@phishkit.local","password":"demo-password","returnSecureToken":true}'
```

Authorized dry-runs only.
