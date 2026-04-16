import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Theme, Screen, OverallStats } from "./types";
import Home from "./components/Home";
import Quiz from "./components/Quiz";
import Settings from "./components/Settings";
import Stats from "./components/Stats";
import "./App.css";

function App() {
  const [screen, setScreen] = useState<Screen>("home");
  const [theme, setTheme] = useState<Theme>(() => {
    return (localStorage.getItem("warframedle-theme") as Theme) || "duviri";
  });
  const [overallStats, setOverallStats] = useState<OverallStats | null>(null);

  const loadOverallStats = () => {
    invoke<OverallStats>("get_overall_stats")
      .then(setOverallStats)
      .catch(console.error);
  };

  useEffect(() => { loadOverallStats(); }, []);
  useEffect(() => { localStorage.setItem("warframedle-theme", theme); }, [theme]);

  const timerEnabled = localStorage.getItem("warframedle-timer") === "true";
  const timerSeconds = parseInt(localStorage.getItem("warframedle-timer-seconds") || "15", 10);

  const handlePlay = async () => {
    try {
      await invoke("start_quiz", { timerEnabled, timerSeconds });
      setScreen("quiz");
    } catch (e) { console.error("Failed to start quiz:", e); }
  };

  const handleQuitQuiz = async () => {
    try { await invoke("end_quiz"); } catch (_) { /* session may already be ended */ }
    loadOverallStats();
    setScreen("home");
  };

  return (
    <div className="app" data-theme={theme}>
      {screen === "home" && (
        <Home
          stats={overallStats}
          onPlay={handlePlay}
          onSettings={() => setScreen("settings")}
          onStats={() => setScreen("stats")}
        />
      )}
      {screen === "quiz" && (
        <Quiz
          timerEnabled={timerEnabled}
          timerSeconds={timerSeconds}
          onQuit={handleQuitQuiz}
        />
      )}
      {screen === "settings" && (
        <Settings
          theme={theme}
          onThemeChange={setTheme}
          onBack={() => setScreen("home")}
        />
      )}
      {screen === "stats" && (
        <Stats
          onBack={() => { loadOverallStats(); setScreen("home"); }}
        />
      )}
    </div>
  );
}

export default App;
