import { OverallStats } from "../types";

interface HomeProps {
  stats: OverallStats | null;
  onPlay: () => void;
  onSettings: () => void;
  onStats: () => void;
}

export default function Home({ stats: _stats, onPlay, onSettings, onStats }: HomeProps) {
  return (
    <div className="home">
      <p>Home — placeholder</p>
      <button className="play-btn" onClick={onPlay}>Play</button>
      <button className="link-btn" onClick={onSettings}>Settings</button>
      <button className="link-btn" onClick={onStats}>Stats</button>
    </div>
  );
}
