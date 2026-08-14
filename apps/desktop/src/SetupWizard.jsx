import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

const PERSONAS = [
  { id: "businessOwner", label: "Business Owner", hint: "Guided campaigns and clear results." },
  { id: "developer", label: "Developer", hint: "Full controls and kit paths visible." },
  { id: "penetrationTester", label: "Penetration Tester", hint: "Advanced proxy, community packs, export tools." },
  { id: "cybersecStudent", label: "Cybersec Student", hint: "Educational defaults and short tutorial." },
];

const STEPS = ["welcome", "storage", "persona", "tutorial"];

export default function SetupWizard({ onComplete, append }) {
  const [step, setStep] = useState(0);
  const [busy, setBusy] = useState(false);
  const [storageMode, setStorageMode] = useState("persistent");
  const [customDataDir, setCustomDataDir] = useState("");
  const [persona, setPersona] = useState("cybersecStudent");
  const [wantTutorial, setWantTutorial] = useState(true);

  const id = STEPS[step];

  const finish = async () => {
    setBusy(true);
    try {
      const config = {
        setupComplete: true,
        persona,
        tutorialCompleted: !wantTutorial,
        storageMode,
        customDataDir: customDataDir.trim() || null,
        kitRootOverride: null,
        ephemeralId: null,
      };
      const saved = await invoke("cmd_complete_setup", { config });
      append?.(
        storageMode === "ephemeral"
          ? "Setup complete · ephemeral mode (wiped next launch)"
          : `Setup complete · data at OS app-data path`
      );
      onComplete?.(saved, { startTutorial: wantTutorial });
    } catch (e) {
      append?.(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="setup-wizard" data-testid="setup-wizard">
      <div className="setup-card">
        <p className="setup-brand">phishkit</p>
        {id === "welcome" && (
          <>
            <h1>Welcome</h1>
            <p className="muted">
              Authorized AiTM and awareness assessments only. You will choose how data is stored,
              who you are in the product, and whether to take a short tutorial.
            </p>
            <p className="muted small">
              Use only with explicit written authorization from the owner of the systems and people
              you assess.
            </p>
            <button type="button" data-testid="setup-next" onClick={() => setStep(1)}>
              Continue
            </button>
          </>
        )}
        {id === "storage" && (
          <>
            <h1>Storage</h1>
            <label className="setup-choice">
              <input
                type="radio"
                name="storage"
                data-testid="setup-storage-persistent"
                checked={storageMode === "persistent"}
                onChange={() => setStorageMode("persistent")}
              />
              <span>
                <strong>Persistent</strong>
                <span className="muted small">
                  Keep assessments in the OS app-data directory (recommended).
                </span>
              </span>
            </label>
            <label className="setup-choice">
              <input
                type="radio"
                name="storage"
                data-testid="setup-storage-ephemeral"
                checked={storageMode === "ephemeral"}
                onChange={() => setStorageMode("ephemeral")}
              />
              <span>
                <strong>Ephemeral</strong>
                <span className="muted small">
                  Sandbox in a temp directory; wiped when you quit and on next launch.
                </span>
              </span>
            </label>
            {storageMode === "persistent" && (
              <label className="field">
                Custom data folder (optional)
                <input
                  value={customDataDir}
                  onChange={(e) => setCustomDataDir(e.target.value)}
                  placeholder="Leave blank for the OS default"
                />
              </label>
            )}
            <div className="row">
              <button type="button" className="ghost" onClick={() => setStep(0)}>
                Back
              </button>
              <button type="button" data-testid="setup-next" onClick={() => setStep(2)}>
                Continue
              </button>
            </div>
          </>
        )}
        {id === "persona" && (
          <>
            <h1>How will you use phishkit?</h1>
            <p className="muted small">This sets default UI density. You can change it in Settings.</p>
            {PERSONAS.map((p) => (
              <label key={p.id} className="setup-choice">
                <input
                type="radio"
                name="persona"
                data-testid={`setup-persona-${p.id}`}
                checked={persona === p.id}
                onChange={() => setPersona(p.id)}
              />
                <span>
                  <strong>{p.label}</strong>
                  <span className="muted small">{p.hint}</span>
                </span>
              </label>
            ))}
            <div className="row">
              <button type="button" className="ghost" onClick={() => setStep(1)}>
                Back
              </button>
              <button type="button" data-testid="setup-next" onClick={() => setStep(3)}>
                Continue
              </button>
            </div>
          </>
        )}
        {id === "tutorial" && (
          <>
            <h1>Tutorial</h1>
            <label className="setup-choice">
              <input
                type="checkbox"
                data-testid="setup-tutorial"
                checked={wantTutorial}
                onChange={(e) => setWantTutorial(e.target.checked)}
              />
              <span>
                <strong>Start the interactive demo tour</strong>
                <span className="muted small">You can replay it anytime from Settings.</span>
              </span>
            </label>
            <div className="row">
              <button type="button" className="ghost" onClick={() => setStep(2)}>
                Back
              </button>
              <button
                type="button"
                data-testid="setup-finish"
                disabled={busy}
                onClick={finish}
              >
                {busy ? "Saving…" : "Finish setup"}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
