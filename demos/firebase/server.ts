/**
 * Firebase-shaped auth mock for local phishkit dry-runs and replay exercises.
 * http://127.0.0.1:9081 — creds: demo@phishkit.local / demo-password
 */
import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import { createHash, randomBytes } from "node:crypto";

const HOST = process.env.HOST ?? "127.0.0.1";
const PORT = Number(process.env.PORT ?? 9081);
const DEMO_USER = "demo@phishkit.local";
const DEMO_PASS = "demo-password";
const DEMO_LOCAL_ID = "demoLocalId001";
const FAKE_API_KEY = "AIzaSyDemoPhishkitFirebaseKey0000001";

type UserRec = {
  email: string;
  localId: string;
  idToken: string;
  refreshToken: string;
  expiresIn: string;
};

const users = new Map<string, UserRec>();

function b64url(data: Buffer | string): string {
  const buf = typeof data === "string" ? Buffer.from(data) : data;
  return buf.toString("base64url");
}

function fakeJwt(email: string, localId: string, ttl = 3600): string {
  const header = b64url(JSON.stringify({ alg: "none", typ: "JWT" }));
  const now = Math.floor(Date.now() / 1000);
  const payload = b64url(
    JSON.stringify({
      iss: "https://securetoken.google.com/phishkit-demo",
      aud: "phishkit-demo",
      auth_time: now,
      user_id: localId,
      sub: localId,
      iat: now,
      exp: now + ttl,
      email,
      email_verified: true,
      firebase: { sign_in_provider: "password" },
    }),
  );
  const sig = b64url(createHash("sha256").update(`${email}:${now}`).digest().subarray(0, 16));
  return `${header}.${payload}.${sig}`;
}

function issueTokens(email: string, localId = DEMO_LOCAL_ID): UserRec {
  const refreshToken = randomBytes(24).toString("base64url");
  const idToken = fakeJwt(email, localId);
  const rec: UserRec = {
    email,
    localId,
    idToken,
    refreshToken,
    expiresIn: "3600",
  };
  users.set(refreshToken, rec);
  return rec;
}

const INDEX_HTML = `<!DOCTYPE html>
<html lang="en"><head>
<meta charset="utf-8"/><meta name="viewport" content="width=device-width, initial-scale=1"/>
<title>phishkit demo firebase — login</title>
<style>
:root{--bg:#101418;--panel:#1b222c;--text:#eef2f7;--muted:#9aa7b8;--accent:#ffca28;--accent-text:#1a1400;--border:#2c3644;--ok:#3dd68c}
*{box-sizing:border-box}body{margin:0;min-height:100vh;font-family:"Source Sans 3","Segoe UI",sans-serif;background:linear-gradient(160deg,#1a2430 0%,#101418 45%,#162018 100%);color:var(--text);display:grid;place-items:center;padding:2rem}
main{width:min(440px,100%);background:var(--panel);border:1px solid var(--border);padding:2rem;border-radius:12px}
h1{margin:0 0 .35rem;font-size:1.35rem}p{margin:0 0 1.1rem;color:var(--muted);line-height:1.45}
label{display:block;font-size:.85rem;margin:.75rem 0 .35rem;color:var(--muted)}
input{width:100%;padding:.65rem .75rem;border-radius:8px;border:1px solid var(--border);background:#0f141b;color:var(--text);font:inherit}
button{margin-top:1.15rem;width:100%;padding:.75rem;border:0;border-radius:8px;background:var(--accent);color:var(--accent-text);font:inherit;font-weight:700;cursor:pointer}
.creds,.out{margin-top:1.1rem;padding:.75rem .9rem;border-radius:8px;background:#121820;border:1px dashed var(--border);font-size:.85rem;color:var(--muted);word-break:break-all}
.creds code,.out code{color:var(--ok)}.err{color:#ff8b8b}.hidden{display:none}.ok{color:var(--ok)}
</style></head><body><main>
<h1>Demo Firebase auth</h1>
<p>Identity Toolkit-shaped mock. Tokens land in <code>localStorage</code> for capture/replay drills.</p>
<div id="login-view">
<p id="err" class="err hidden"></p>
<label for="email">Email</label>
<input id="email" type="email" value="${DEMO_USER}"/>
<label for="password">Password</label>
<input id="password" type="password" value="${DEMO_PASS}"/>
<button id="signin" type="button">Sign in with email</button>
<div class="creds">Test credentials:<br/><code>${DEMO_USER}</code> / <code>${DEMO_PASS}</code><br/>Fake API key: <code>${FAKE_API_KEY}</code></div>
</div>
<div id="dash-view" class="hidden">
<p class="ok">Signed in. Tokens stored under <code>firebase:authUser</code>.</p>
<div class="out" id="token-out"></div>
<button id="signout" type="button" style="background:#3d8bfd;color:#fff;margin-top:1rem;">Sign out</button>
</div>
</main>
<script>
const API_KEY = ${JSON.stringify(FAKE_API_KEY)};
const STORAGE_KEY = 'firebase:authUser:' + API_KEY + ':[DEFAULT]';
function showDash(user) {
  document.getElementById('login-view').classList.add('hidden');
  document.getElementById('dash-view').classList.remove('hidden');
  document.getElementById('token-out').innerHTML =
    'email: <code>' + user.email + '</code><br/>' +
    'localId: <code>' + user.localId + '</code><br/>' +
    'id_token: <code>' + user.idToken.slice(0, 48) + '…</code><br/>' +
    'refresh_token: <code>' + user.refreshToken.slice(0, 24) + '…</code>';
}
const existing = localStorage.getItem(STORAGE_KEY);
if (existing) { try { showDash(JSON.parse(existing)); } catch (e) {} }
document.getElementById('signin').onclick = async () => {
  const err = document.getElementById('err');
  err.classList.add('hidden');
  const email = document.getElementById('email').value;
  const password = document.getElementById('password').value;
  const url = '/identitytoolkit.googleapis.com/v1/accounts:signInWithPassword?key=' + API_KEY;
  const res = await fetch(url, { method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password, returnSecureToken: true }) });
  const data = await res.json();
  if (!res.ok) { err.textContent = (data.error && data.error.message) || 'sign-in failed'; err.classList.remove('hidden'); return; }
  const authUser = { uid: data.localId, email: data.email, emailVerified: true,
    stsTokenManager: { apiKey: API_KEY, refreshToken: data.refreshToken, accessToken: data.idToken,
      expirationTime: Date.now() + (parseInt(data.expiresIn, 10) * 1000) },
    apiKey: API_KEY, appName: '[DEFAULT]', localId: data.localId, idToken: data.idToken, refreshToken: data.refreshToken };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(authUser));
  localStorage.setItem('id_token', data.idToken);
  localStorage.setItem('refresh_token', data.refreshToken);
  showDash(data);
};
document.getElementById('signout').onclick = () => {
  localStorage.removeItem(STORAGE_KEY); localStorage.removeItem('id_token'); localStorage.removeItem('refresh_token');
  location.reload();
};
</script></body></html>`;

function sendJson(res: ServerResponse, code: number, payload: unknown) {
  const body = Buffer.from(JSON.stringify(payload) + "\n");
  res.writeHead(code, {
    "Content-Type": "application/json; charset=utf-8",
    "Content-Length": body.length,
    "Access-Control-Allow-Origin": "*",
    "Cache-Control": "no-store",
  });
  res.end(body);
}

function sendHtml(res: ServerResponse, code: number, html: string) {
  const body = Buffer.from(html);
  res.writeHead(code, {
    "Content-Type": "text/html; charset=utf-8",
    "Content-Length": body.length,
    "Cache-Control": "no-store",
  });
  res.end(body);
}

function readBody(req: IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = [];
    req.on("data", (c) => chunks.push(c));
    req.on("end", () => resolve(Buffer.concat(chunks).toString("utf8")));
    req.on("error", reject);
  });
}

const server = createServer(async (req, res) => {
  const url = new URL(req.url ?? "/", `http://${req.headers.host ?? "localhost"}`);
  const path = url.pathname;
  console.log(`[demo-firebase] ${req.socket.remoteAddress} ${req.method} ${path}`);

  if (req.method === "OPTIONS") {
    res.writeHead(204, {
      "Access-Control-Allow-Origin": "*",
      "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
      "Access-Control-Allow-Headers": "Content-Type, Authorization",
    });
    res.end();
    return;
  }

  if (req.method === "GET") {
    if (path === "/" || path === "/login") {
      sendHtml(res, 200, INDEX_HTML);
      return;
    }
    if (path === "/healthz") {
      sendJson(res, 200, { ok: true, apiKey: FAKE_API_KEY });
      return;
    }
    sendJson(res, 404, { error: { message: "NOT_FOUND", code: 404 } });
    return;
  }

  if (req.method !== "POST") {
    sendJson(res, 404, { error: { message: "NOT_FOUND", code: 404 } });
    return;
  }

  let data: Record<string, unknown> = {};
  try {
    data = JSON.parse((await readBody(req)) || "{}") as Record<string, unknown>;
  } catch {
    data = {};
  }

  if (path.endsWith("/accounts:signInWithPassword") || path === "/api/token") {
    const email = String(data.email ?? data.username ?? "").trim();
    const password = String(data.password ?? "");
    if (email !== DEMO_USER || password !== DEMO_PASS) {
      sendJson(res, 400, { error: { code: 400, message: "INVALID_PASSWORD", errors: [] } });
      return;
    }
    const tok = issueTokens(email);
    if (path === "/api/token") {
      sendJson(res, 200, {
        access_token: tok.idToken,
        id_token: tok.idToken,
        refresh_token: tok.refreshToken,
        local_id: tok.localId,
        expires_in: tok.expiresIn,
        token_type: "Bearer",
      });
      return;
    }
    sendJson(res, 200, {
      kind: "identitytoolkit#VerifyPasswordResponse",
      localId: tok.localId,
      email: tok.email,
      displayName: "Demo User",
      idToken: tok.idToken,
      registered: true,
      refreshToken: tok.refreshToken,
      expiresIn: tok.expiresIn,
    });
    return;
  }

  if (path.endsWith("/token") || path === "/securetoken.googleapis.com/v1/token") {
    const refresh = String(data.refresh_token ?? data.refreshToken ?? "");
    const grant = String(data.grant_type ?? "");
    if (grant && grant !== "refresh_token") {
      sendJson(res, 400, { error: "unsupported_grant_type" });
      return;
    }
    const rec = users.get(refresh);
    if (!rec) {
      sendJson(res, 400, { error: { message: "INVALID_REFRESH_TOKEN" } });
      return;
    }
    const newId = fakeJwt(rec.email, rec.localId);
    rec.idToken = newId;
    sendJson(res, 200, {
      access_token: newId,
      expires_in: "3600",
      token_type: "Bearer",
      refresh_token: refresh,
      id_token: newId,
      user_id: rec.localId,
      project_id: "phishkit-demo",
    });
    return;
  }

  if (path.endsWith("/accounts:lookup")) {
    sendJson(res, 200, {
      users: [{ localId: DEMO_LOCAL_ID, email: DEMO_USER, emailVerified: true }],
    });
    return;
  }

  sendJson(res, 404, { error: { message: "NOT_FOUND", code: 404 } });
});

server.listen(PORT, HOST, () => {
  console.log(`demo-firebase listening on http://${HOST}:${PORT}`);
  console.log(`  login:  http://${HOST}:${PORT}/login`);
  console.log(`  creds:  ${DEMO_USER} / ${DEMO_PASS}`);
  console.log(`  key:    ${FAKE_API_KEY}`);
});
