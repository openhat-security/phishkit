import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Hint from "./components/Hint";

const PERSONAS = [
  { id: "businessOwner", label: "Business Owner" },
  { id: "developer", label: "Developer" },
  { id: "penetrationTester", label: "Penetration Tester" },
  { id: "cybersecStudent", label: "Cybersec Student" },
];

export default function SettingsView({ append, onStartTutorial, onSetupChanged }) {
  const [info, setInfo] = useState(null);
  const [busy, setBusy] = useState("");

  const load = async () => {
    try {
      setInfo(await invoke("cmd_paths_info"));
    } catch (e) {
      append?.(String(e));
    }
  };

  useEffect(() => {
    load();
  }, []);

  const savePersona = async (persona) => {
    setBusy("persona");
    try {
      const cfg = await invoke("cmd_get_setup");
      await invoke("cmd_complete_setup", {
        config: { ...cfg, persona, setupComplete: true },
      });
      append?.(`Persona set to ${persona}`);
      await load();
      onSetupChanged?.();
    } catch (e) {
      append?.(String(e));
    } finally {
      setBusy("");
    }
  };

  const resetTutorial = async () => {
    setBusy("tutorial");
    try {
      await invoke("cmd_set_tutorial_completed", { done: false });
      append?.("Tutorial marked incomplete — starting tour");
      onStartTutorial?.();
      await load();
    } catch (e) {
      append?.(String(e));
    } finally {
      setBusy("");
    }
  };

  if (!info) {
    return <div className="muted">Loading settings…</div>;
  }

  return (
    <div className="settings-view" data-testid="settings-view">
      <section className="card">
        <h2 className="section-head-title">
          Settings
          <Hint hint="Durable preferences live in the OS config directory. Assessment data lives in the OS data directory (or an ephemeral sandbox)." />
        </h2>
        <div className="settings-grid">
          <div>
            <strong>Storage mode</strong>
            <p className="muted small">{info.storageMode}</p>
          </div>
          <div>
            <strong>Config</strong>
            <p className="muted small mono">{info.configDir}</p>
          </div>
          <div>
            <strong>Data</strong>
            <p className="muted small mono">{info.dataDir}</p>
          </div>
          <div>
            <strong>Database</strong>
            <p className="muted small mono">{info.dbPath}</p>
          </div>
          <div>
            <strong>Kit root</strong>
            <p className="muted small mono">{info.kitRoot}</p>
          </div>
        </div>
      </section>

      <section className="card">
        <h3>Persona</h3>
        <div className="row wrap">
          {PERSONAS.map((p) => (
            <button
              key={p.id}
              type="button"
              className={info.persona === p.id ? "" : "ghost"}
              disabled={!!busy}
              onClick={() => savePersona(p.id)}
            >
              {p.label}
            </button>
          ))}
        </div>
      </section>

      <section className="card">
        <h3>Tutorial</h3>
        <p className="muted small">
          Status: {info.tutorialCompleted ? "completed / skipped" : "not completed"}
        </p>
        <button type="button" disabled={!!busy} onClick={resetTutorial}>
          Replay tutorial
        </button>
      </section>
    </div>
  );
}
