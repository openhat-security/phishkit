import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Hint from "./components/Hint";
import EmptyState from "./components/EmptyState";

/** Annotated shortlist chips — mirrors demos/community/README.md */
export const COMMUNITY_SHORTLIST = [
  { q: "okta", label: "Okta" },
  { q: "o365", label: "O365" },
  { q: "microsoft", label: "Microsoft" },
  { q: "google", label: "Google" },
  { q: "onelogin", label: "OneLogin" },
  { q: "adfs", label: "ADFS" },
];

function formatBytes(n) {
  const b = Number(n) || 0;
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / (1024 * 1024)).toFixed(1)} MB`;
}

export default function CommunityPhishlets({
  busy,
  setBusy,
  append,
  kit,
  onImported,
}) {
  const [rows, setRows] = useState([]);
  const [active, setActive] = useState([]);
  const [q, setQ] = useState("");
  const [loaded, setLoaded] = useState(false);

  const refreshActive = useCallback(async () => {
    try {
      setActive(await invoke("list_active"));
    } catch {
      setActive([]);
    }
  }, []);

  const load = useCallback(
    async (query) => {
      try {
        setRows(await invoke("list_community", { query: query || null }));
      } catch (e) {
        append?.(String(e));
        setRows([]);
      } finally {
        setLoaded(true);
      }
    },
    [append]
  );

  useEffect(() => {
    load("");
    refreshActive();
  }, [load, refreshActive]);

  const run = async (label, fn) => {
    setBusy?.(label);
    try {
      return await fn();
    } catch (e) {
      append?.(`${label}: ${e}`);
      throw e;
    } finally {
      setBusy?.("");
    }
  };

  const onSync = async () => {
    const r = await run("sync", () => invoke("sync_community"));
    append?.(
      `Synced ${r.merged_count} community phishlets` +
        (r.collision_count ? ` · ${r.collision_count} collisions` : "")
    );
    await load(q);
    await refreshActive();
  };

  const onImport = async (name) => {
    const r = await run("import", () => invoke("import_community", { name }));
    append?.(`Imported ${r.name}`);
    await refreshActive();
    await onImported?.(r);
  };

  const activeSet = new Set(
    (active || []).map((n) => String(n).replace(/\.ya?ml$/i, "").toLowerCase())
  );
  const indexMissing =
    kit &&
    (kit.communityIndex === false || kit.community_index === false);

  return (
    <section className="card community-phishlets" data-testid="community-view">
      <div className="section-head">
        <h2 className="section-head-title">
          Community phishlets
          <Hint hint="Third-party YAML packs vendored under vendor/community-phishlets/. Use them to learn cookie/SSO/OAuth patterns on authorized targets only — not as a menu of live brands to attack." />
        </h2>
      </div>

      <p className="muted small">
        Authorized targets only. Packs ship in-repo; <strong>Sync packs</strong> refreshes
        pinned sources. Prefer localhost demos for first-run practice.
      </p>

      {indexMissing ? (
        <EmptyState
          compact
          title="Community index missing"
          action={
            <button type="button" disabled={!!busy} onClick={onSync}>
              Sync packs
            </button>
          }
        >
          Run Sync packs or <span className="mono">make community-phishlets</span> to
          populate vendor/community-phishlets/index.json.
        </EmptyState>
      ) : (
        <>
          <div className="row community-toolbar">
            <button
              type="button"
              data-testid="community-sync"
              disabled={!!busy}
              onClick={onSync}
            >
              {busy === "sync" ? "Syncing…" : "Sync packs"}
            </button>
            <input
              type="search"
              data-testid="community-search"
              className="grow"
              placeholder="Filter o365, okta, google…"
              value={q}
              onChange={(e) => {
                const next = e.target.value;
                setQ(next);
                load(next);
              }}
            />
          </div>

          <div className="chips community-shortlist">
            {COMMUNITY_SHORTLIST.map((c) => (
              <button
                key={c.q}
                type="button"
                className={`ghost chip${q === c.q ? " active" : ""}`}
                data-testid={`community-chip-${c.q}`}
                disabled={!!busy}
                onClick={() => {
                  setQ(c.q);
                  load(c.q);
                }}
              >
                {c.label}
              </button>
            ))}
            {q ? (
              <button
                type="button"
                className="ghost chip"
                onClick={() => {
                  setQ("");
                  load("");
                }}
              >
                Clear
              </button>
            ) : null}
          </div>

          {!loaded ? (
            <p className="muted">Loading catalog…</p>
          ) : !rows.length ? (
            <p className="muted">No phishlets match “{q || "…"}”.</p>
          ) : (
            <ul className="list community-list">
              {rows.slice(0, 60).map((p) => {
                const stem = String(p.name || "").replace(/\.ya?ml$/i, "");
                const imported = activeSet.has(stem.toLowerCase());
                return (
                  <li key={`${p.source}:${p.name}`}>
                    <span className="mono truncate" title={p.path}>
                      {p.name}
                    </span>
                    <span className="tag">{p.source}</span>
                    <span className="muted small">{formatBytes(p.bytes)}</span>
                    {imported ? <span className="tag status-active">imported</span> : null}
                    <button
                      type="button"
                      data-testid={`community-import-${stem}`}
                      disabled={!!busy}
                      onClick={() => onImport(p.name)}
                    >
                      {imported ? "Re-import" : "Import"}
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
          {rows.length > 60 ? (
            <p className="muted small">Showing 60 of {rows.length} — refine the filter.</p>
          ) : null}
        </>
      )}
    </section>
  );
}
