/**
 * Cookie-session practice app for local phishkit dry-runs.
 * http://127.0.0.1:9080 — creds: demo@phishkit.local / demo-password
 */
import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import { createHmac, randomBytes } from "node:crypto";
import { parse as parseQuery } from "node:querystring";

const HOST = process.env.HOST ?? "127.0.0.1";
const PORT = Number(process.env.PORT ?? 9080);
const DEMO_USER = "demo@phishkit.local";
const DEMO_PASS = "demo-password";
const COOKIE_NAME = "session";
const SECRET = Buffer.from("phishkit-demo-cookie-dev-only");

const sessions = new Map<string, string>();

function sign(token: string): string {
  return createHmac("sha256", SECRET).update(token).digest("hex").slice(0, 16);
}

function makeSession(user: string): string {
  const raw = randomBytes(18).toString("base64url");
  const token = `${raw}.${sign(raw)}`;
  sessions.set(token, user);
  return token;
}

function parseCookies(header: string | undefined): Record<string, string> {
  const out: Record<string, string> = {};
  if (!header) return out;
  for (const part of header.split(";")) {
    const [k, ...rest] = part.trim().split("=");
    if (k) out[k] = rest.join("=");
  }
  return out;
}

function sessionUser(req: IncomingMessage): string | undefined {
  const token = parseCookies(req.headers.cookie)[COOKIE_NAME];
  return token ? sessions.get(token) : undefined;
}

const LOGIN_HTML = `<!DOCTYPE html>
<html lang="en"><head>
<meta charset="utf-8"/><meta name="viewport" content="width=device-width, initial-scale=1"/>
<title>phishkit cookie-session demo — login</title>
<style>
:root{--bg:#0f1419;--panel:#1a2332;--text:#e7ecf3;--muted:#9aa7b8;--accent:#3d8bfd;--border:#2a3648;--ok:#3dd68c}
*{box-sizing:border-box}body{margin:0;min-height:100vh;font-family:"IBM Plex Sans","Segoe UI",sans-serif;background:radial-gradient(1200px 600px at 10% -10%,#1c2a44,transparent),radial-gradient(900px 500px at 100% 0%,#152018,transparent),var(--bg);color:var(--text);display:grid;place-items:center;padding:2rem}
main{width:min(420px,100%);background:var(--panel);border:1px solid var(--border);padding:2rem;border-radius:12px}
h1{margin:0 0 .35rem;font-size:1.35rem}p{margin:0 0 1.25rem;color:var(--muted);font-size:.95rem;line-height:1.45}
label{display:block;font-size:.85rem;margin:.75rem 0 .35rem;color:var(--muted)}
input{width:100%;padding:.65rem .75rem;border-radius:8px;border:1px solid var(--border);background:#0f1622;color:var(--text);font:inherit}
button{margin-top:1.25rem;width:100%;padding:.75rem;border:0;border-radius:8px;background:var(--accent);color:#fff;font:inherit;font-weight:600;cursor:pointer}
.creds{margin-top:1.25rem;padding:.75rem .9rem;border-radius:8px;background:#121a27;border:1px dashed var(--border);font-size:.85rem;color:var(--muted)}
.creds code{color:var(--ok)}.err{color:#ff8b8b;font-size:.9rem;margin-bottom:.75rem}
</style></head><body><main>
<h1>Cookie-session demo</h1>
<p>Cookie-session login mock for authorized phishkit dry-runs.</p>
{{error}}
<form method="post" action="/login" autocomplete="on">
<label for="username">Email</label>
<input id="username" name="username" type="email" required value="${DEMO_USER}"/>
<label for="password">Password</label>
<input id="password" name="password" type="password" required value="${DEMO_PASS}"/>
<button type="submit">Sign in</button>
</form>
<div class="creds">Test credentials:<br/><code>${DEMO_USER}</code> / <code>${DEMO_PASS}</code></div>
</main></body></html>`;

function dashboardHtml(user: string): string {
  return `<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><meta name="viewport" content="width=device-width, initial-scale=1"/>
<title>phishkit cookie-session demo — dashboard</title>
<style>body{margin:0;font-family:"IBM Plex Sans","Segoe UI",sans-serif;background:#0f1419;color:#e7ecf3;min-height:100vh;display:grid;place-items:center}
main{width:min(520px,92vw);background:#1a2332;border:1px solid #2a3648;border-radius:12px;padding:2rem}
h1{margin-top:0}.ok{color:#3dd68c}a{color:#3d8bfd}code{background:#121a27;padding:.15rem .4rem;border-radius:4px}</style>
</head><body><main>
<h1 class="ok">Signed in</h1>
<p>Session cookie <code>session</code> is set for <strong>${user}</strong>.</p>
<p>This is what a cookie-SSO capture should harvest in evilginx.</p>
<p><a href="/logout">Sign out</a></p>
</main></body></html>`;
}

function send(
  res: ServerResponse,
  code: number,
  body: string | Buffer,
  contentType = "text/html; charset=utf-8",
  headers: Record<string, string> = {},
) {
  const buf = Buffer.isBuffer(body) ? body : Buffer.from(body);
  res.writeHead(code, {
    "Content-Type": contentType,
    "Content-Length": buf.length,
    "Cache-Control": "no-store",
    ...headers,
  });
  res.end(buf);
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
  console.log(`[demo-cookie] ${req.socket.remoteAddress} ${req.method} ${path}`);

  if (req.method === "GET" || (req.method === "POST" && path === "/dashboard")) {
    if (path === "/" || path === "/login") {
      const user = sessionUser(req);
      if (user) {
        res.writeHead(302, { Location: "/dashboard" });
        res.end();
        return;
      }
      send(res, 200, LOGIN_HTML.replace("{{error}}", ""));
      return;
    }
    if (path === "/dashboard") {
      const user = sessionUser(req);
      if (!user) {
        res.writeHead(302, { Location: "/login" });
        res.end();
        return;
      }
      send(res, 200, dashboardHtml(user));
      return;
    }
    if (path === "/logout") {
      const token = parseCookies(req.headers.cookie)[COOKIE_NAME];
      if (token) sessions.delete(token);
      res.writeHead(302, {
        Location: "/login",
        "Set-Cookie": `${COOKIE_NAME}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax`,
      });
      res.end();
      return;
    }
    if (path === "/healthz") {
      send(res, 200, '{"ok":true}\n', "application/json");
      return;
    }
    send(res, 404, "not found\n", "text/plain; charset=utf-8");
    return;
  }

  if (req.method === "POST" && path === "/login") {
    const raw = await readBody(req);
    const form = parseQuery(raw);
    const user = String(form.username ?? "").trim();
    const password = String(form.password ?? "");
    if (user !== DEMO_USER || password !== DEMO_PASS) {
      send(
        res,
        401,
        LOGIN_HTML.replace("{{error}}", '<p class="err">Invalid email or password.</p>'),
      );
      return;
    }
    const token = makeSession(user);
    res.writeHead(302, {
      Location: "/dashboard",
      "Set-Cookie": `${COOKIE_NAME}=${token}; Path=/; HttpOnly; SameSite=Lax`,
    });
    res.end();
    return;
  }

  send(res, 404, "not found\n", "text/plain; charset=utf-8");
});

server.listen(PORT, HOST, () => {
  console.log(`demo-cookie listening on http://${HOST}:${PORT}`);
  console.log(`  login:  http://${HOST}:${PORT}/login`);
  console.log(`  creds:  ${DEMO_USER} / ${DEMO_PASS}`);
});
