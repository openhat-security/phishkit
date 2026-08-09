import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

const AUP_TEXT = `Authorized use only.

You confirm that:
• You have written authorization from the organization that owns the target systems and mailboxes.
• Recipients and domains are in scope for this engagement.
• You will use BYO mail credentials you control and accept deliverability / ESP AUP risk.
• This tool must not be used for unauthorized phishing or spam.

phishkit does not host mail or guarantee inbox placement.`;

export default function AupGate({ onAccepted }) {
  const [status, setStatus] = useState(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");

  const refresh = () =>
    invoke("get_aup_status")
      .then((s) => {
        setStatus(s);
        if (s.accepted && onAccepted) onAccepted(s);
      })
      .catch((e) => setErr(String(e)));

  useEffect(() => {
    refresh();
  }, []);

  if (status?.accepted) {
    return (
      <p className="muted small aup-ok">
        Authorized-use acknowledgment on file
        {status.acceptedAt || status.accepted_at
          ? ` (${status.acceptedAt || status.accepted_at})`
          : ""}
      </p>
    );
  }

  if (!status) {
    return <p className="muted">Checking authorization acknowledgment…</p>;
  }

  return (
    <div className="aup-gate">
      <h3>Authorization required</h3>
      <pre className="aup-text">{AUP_TEXT}</pre>
      {err && <div className="warn-banner">{err}</div>}
      <button
        data-testid="aup-accept"
        disabled={busy}
        onClick={async () => {
          setBusy(true);
          setErr("");
          try {
            const s = await invoke("accept_aup");
            setStatus(s);
            if (onAccepted) onAccepted(s);
          } catch (e) {
            setErr(String(e));
          } finally {
            setBusy(false);
          }
        }}
      >
        I have written authorization — continue
      </button>
    </div>
  );
}
