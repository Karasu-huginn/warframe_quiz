interface TopBarProps {
  title: string;
  onBack: () => void;
}

export default function TopBar({ title, onBack }: TopBarProps) {
  return (
    <div className="top-bar">
      <button className="back-btn" onClick={onBack}>&larr;</button>
      <span className="top-bar-title">{title}</span>
    </div>
  );
}
