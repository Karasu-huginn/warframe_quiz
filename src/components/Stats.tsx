import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { OverallStats, RecentSession } from "../types";
import TopBar from "./TopBar";

interface StatsProps {
  onBack: () => void;
}

export default function Stats({ onBack }: StatsProps) {
  const [overall, setOverall] = useState<OverallStats | null>(null);
  const [sessions, setSessions] = useState<RecentSession[]>([]);

  useEffect(() => {
    invoke<OverallStats>("get_overall_stats").then(setOverall).catch(console.error);
    invoke<RecentSession[]>("get_recent_sessions", { limit: 10 }).then(setSessions).catch(console.error);
  }, []);

  const accuracy = overall && overall.total_answered > 0
    ? Math.round((overall.total_correct / overall.total_answered) * 100)
    : 0;

  return (
    <div className="stats-screen">
      <TopBar title="Statistiques" onBack={onBack} />

      {overall && (
        <div className="stats-summary">
          <div className="stat-card">
            <div className="stat-value">{overall.total_games}</div>
            <div className="stat-label">Parties jouées</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">{overall.best_streak}</div>
            <div className="stat-label">Meilleure série</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">{accuracy}%</div>
            <div className="stat-label">Précision globale</div>
          </div>
        </div>
      )}

      <h3 className="section-title">Parties récentes</h3>
      {sessions.length === 0 ? (
        <p className="no-data">Aucune partie enregistrée</p>
      ) : (
        <div className="sessions-list">
          {sessions.map((s) => (
            <div key={s.id} className="session-row">
              <span className="session-date">{s.started_at}</span>
              <span className="session-score">{s.score}/{s.total_questions}</span>
              <span className="session-streak">🔥 {s.best_streak}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
