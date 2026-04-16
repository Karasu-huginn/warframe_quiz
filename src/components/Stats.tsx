interface StatsProps {
  onBack: () => void;
}

export default function Stats({ onBack }: StatsProps) {
  return (
    <div className="stats-screen">
      <button className="back-btn" onClick={onBack}>Back</button>
      <p>Stats — placeholder</p>
    </div>
  );
}
