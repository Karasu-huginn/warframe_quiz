import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Theme, FetchProgress } from "../types";
import TopBar from "./TopBar";

interface SettingsProps {
  theme: Theme;
  onThemeChange: (t: Theme) => void;
  onBack: () => void;
}

const THEMES: { id: Theme; name: string; desc: string }[] = [
  { id: "duviri", name: "Duviri", desc: "Onirique & pictural" },
  { id: "grineer", name: "Grineer", desc: "Industriel & brut" },
  { id: "lotus", name: "Lotus", desc: "Transmission éthérée" },
];

export default function Settings({ theme, onThemeChange, onBack }: SettingsProps) {
  const [timerEnabled, setTimerEnabled] = useState(
    localStorage.getItem("warframedle-timer") === "true"
  );
  const [timerSeconds, setTimerSeconds] = useState(
    parseInt(localStorage.getItem("warframedle-timer-seconds") || "15", 10)
  );
  const [fetching, setFetching] = useState(false);
  const [progress, setProgress] = useState<FetchProgress | null>(null);

  useEffect(() => { localStorage.setItem("warframedle-timer", String(timerEnabled)); }, [timerEnabled]);
  useEffect(() => { localStorage.setItem("warframedle-timer-seconds", String(timerSeconds)); }, [timerSeconds]);

  useEffect(() => {
    const unlisten = listen<FetchProgress>("fetch_progress", (e) => setProgress(e.payload));
    return () => { unlisten.then((f) => f()); };
  }, []);

  const handleFetch = async () => {
    setFetching(true);
    try { await invoke("fetch_wiki_data"); } catch (e) { console.error("Fetch failed:", e); }
    finally { setFetching(false); setProgress(null); }
  };

  return (
    <div className="settings-screen">
      <TopBar title="Paramètres" onBack={onBack} />

      <div className="settings-section">
        <h3 className="section-title">Thème</h3>
        <div className="theme-cards">
          {THEMES.map((t) => (
            <button key={t.id} className={`theme-card ${theme === t.id ? "active" : ""}`} onClick={() => onThemeChange(t.id)}>
              <div className={`theme-swatch theme-swatch-${t.id}`} />
              <div className="theme-name">{t.name}</div>
              <div className="theme-desc">{t.desc}</div>
            </button>
          ))}
        </div>
      </div>

      <div className="settings-section">
        <h3 className="section-title">Chronomètre</h3>
        <label className="toggle-row">
          <span>Activer le chronomètre</span>
          <input type="checkbox" checked={timerEnabled} onChange={(e) => setTimerEnabled(e.target.checked)} />
        </label>
        {timerEnabled && (
          <label className="toggle-row">
            <span>Secondes par question</span>
            <input type="number" min={5} max={60} value={timerSeconds} onChange={(e) => setTimerSeconds(parseInt(e.target.value, 10) || 15)} className="timer-input" />
          </label>
        )}
      </div>

      <div className="settings-section">
        <h3 className="section-title">Langue</h3>
        <label className="toggle-row">
          <span>Français</span>
          <input type="checkbox" checked disabled />
        </label>
      </div>

      <div className="settings-section">
        <h3 className="section-title">Données</h3>
        <button className="fetch-btn" onClick={handleFetch} disabled={fetching}>
          {fetching ? "Mise à jour..." : "Mettre à jour les données"}
        </button>
        {progress && <p className="fetch-progress">{progress.message} ({progress.current}/{progress.total})</p>}
      </div>
    </div>
  );
}
