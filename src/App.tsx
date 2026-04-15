import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface DbStats {
  warframe_count: number;
  ability_count: number;
  weapon_count: number;
  mod_count: number;
}

interface FetchProgress {
  category: string;
  status: string;
  current: number;
  total: number;
  message: string;
}

function App() {
  const [stats, setStats] = useState<DbStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [fetching, setFetching] = useState(false);
  const [progress, setProgress] = useState<FetchProgress | null>(null);

  const loadStats = () => {
    invoke<DbStats>("get_db_stats")
      .then(setStats)
      .catch((e) => setError(String(e)));
  };

  useEffect(() => {
    loadStats();
    const unlisten = listen<FetchProgress>("fetch_progress", (event) => {
      setProgress(event.payload);
    });
    return () => { unlisten.then((f) => f()); };
  }, []);

  const handleFetch = async () => {
    setFetching(true);
    setError(null);
    try {
      await invoke("fetch_wiki_data");
      loadStats();
    } catch (e) {
      setError(String(e));
    } finally {
      setFetching(false);
      setProgress(null);
    }
  };

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
      <button onClick={handleFetch} disabled={fetching}>
        {fetching ? "Fetching..." : "Fetch Wiki Data"}
      </button>
      {progress && (
        <p>{progress.message} ({progress.current}/{progress.total})</p>
      )}
    </div>
  );
}

export default App;
