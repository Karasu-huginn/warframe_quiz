import { OverallStats } from "../types";

interface HomeProps {
  stats: OverallStats | null;
  onPlay: () => void;
  onSettings: () => void;
  onStats: () => void;
}

export default function Home({ stats, onPlay, onSettings, onStats }: HomeProps) {
  const accuracy = stats && stats.total_answered > 0
    ? Math.round((stats.total_correct / stats.total_answered) * 100)
    : 0;

  return (
    <div className="home">
      <div className="logo">Warframedle</div>
      <div className="divider" />

      <button className="play-btn" onClick={onPlay}>Jouer</button>

      <button className="settings-pill" onClick={onSettings}>
        <span className="gear-icon">&#9881;</span> Paramètres
      </button>

      {stats && (
        <div className="stats-row">
          <div className="stat-card">
            <div className="stat-value">{stats.total_games}</div>
            <div className="stat-label">Parties</div>
          </div>
          <div className="stat-divider" />
          <div className="stat-card">
            <div className="stat-value">{stats.best_streak}</div>
            <div className="stat-label">Meilleure série</div>
          </div>
          <div className="stat-divider" />
          <div className="stat-card">
            <div className="stat-value">{accuracy}%</div>
            <div className="stat-label">Précision</div>
          </div>
        </div>
      )}

      <button className="link-btn" onClick={onStats}>
        Voir les statistiques &rarr;
      </button>
    </div>
  );
}
