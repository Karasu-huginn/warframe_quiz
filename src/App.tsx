import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface DbStats {
  warframe_count: number;
  ability_count: number;
  weapon_count: number;
  mod_count: number;
}

function App() {
  const [stats, setStats] = useState<DbStats | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<DbStats>("get_db_stats")
      .then(setStats)
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <div>
      <h1>Warframedle</h1>
      {error && <p style={{ color: "red" }}>Error: {error}</p>}
      {stats && (
        <div>
          <h2>Database Status</h2>
          <ul>
            <li>Warframes: {stats.warframe_count}</li>
            <li>Abilities: {stats.ability_count}</li>
            <li>Weapons: {stats.weapon_count}</li>
            <li>Mods: {stats.mod_count}</li>
          </ul>
        </div>
      )}
      {!stats && !error && <p>Connecting to database...</p>}
    </div>
  );
}

export default App;
