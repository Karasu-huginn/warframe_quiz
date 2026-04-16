import { Theme } from "../types";

interface SettingsProps {
  theme: Theme;
  onThemeChange: (t: Theme) => void;
  onBack: () => void;
}

export default function Settings({ theme: _theme, onThemeChange: _onThemeChange, onBack }: SettingsProps) {
  return (
    <div className="settings-screen">
      <button className="back-btn" onClick={onBack}>Back</button>
      <p>Settings — placeholder</p>
    </div>
  );
}
