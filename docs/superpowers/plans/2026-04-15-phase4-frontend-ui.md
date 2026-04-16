# Phase 4: Frontend UI — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the full React frontend with 3 Warframe-themed visual skins (Duviri, Grineer, Lotus), 5 screens (Home, Quiz, Feedback, Settings, Stats), and connect it to the existing game engine via Tauri IPC.

**Architecture:** Single-page React app with state-based screen routing (no router library). 3 CSS themes via `data-theme` attribute and CSS custom properties. Each screen is a separate component. Quiz state machine: answering → feedback → next question. All game logic stays in Rust — React just renders and calls commands.

**Tech Stack:** React 18, TypeScript, CSS custom properties, Tauri IPC (`@tauri-apps/api`)

> **Phase scope:** Phase 4 of 5. Depends on Phase 3 (game engine commands). Phase 5 (Packaging) comes next.

---

## File Structure

```
src/
├── App.tsx              — theme wrapper + screen router
├── App.css              — 3 theme variable sets + shared styles + theme decorations
├── types.ts             — shared TypeScript interfaces for Tauri commands
├── components/
│   ├── Home.tsx         — logo, play button, quick stats, settings/stats navigation
│   ├── Quiz.tsx         — question display, answers, score bar, feedback state, timer
│   ├── Settings.tsx     — theme picker, timer config, language, data manager
│   ├── Stats.tsx        — historical stats + recent sessions
│   └── TopBar.tsx       — reusable back arrow + title bar
```

Note: Clue rendering is handled inline in Quiz.tsx with a switch on `clue.type`. Separate clue component files are unnecessary for this complexity level — each clue type is 5-15 lines of JSX.

---

### Task 1: Backend — Stats Commands + Schema Migration

**Files:**
- Modify: `src-tauri/src/db/schema.rs` — add best_streak migration
- Modify: `src-tauri/src/game/mod.rs` — save best_streak in end()
- Modify: `src-tauri/src/commands/mod.rs` — add get_overall_stats + get_recent_sessions
- Modify: `src-tauri/src/lib.rs` — register new commands

- [ ] **Step 1: Add best_streak column migration to schema.rs**

After the `execute_batch` call in `create_tables`, add a migration that adds the column if missing:

```rust
// At the end of create_tables, after execute_batch:
let _ = conn.execute(
    "ALTER TABLE quiz_sessions ADD COLUMN best_streak INTEGER NOT NULL DEFAULT 0",
    [],
);
```

The `let _ =` ignores the "duplicate column" error on subsequent runs.

- [ ] **Step 2: Update QuizSession::end() to save best_streak**

In `src-tauri/src/game/mod.rs`, update the `end` method:

```rust
pub fn end(&self, conn: &Connection) -> Result<SessionStats, String> {
    conn.execute(
        "UPDATE quiz_sessions SET score = ?1, total_questions = ?2, best_streak = ?3 WHERE id = ?4",
        params![self.score, self.total, self.best_streak, self.id],
    ).map_err(|e| e.to_string())?;
    Ok(self.stats())
}
```

- [ ] **Step 3: Add stats commands to commands/mod.rs**

```rust
#[derive(Serialize)]
pub struct OverallStats {
    pub total_games: i64,
    pub best_streak: i64,
    pub total_correct: i64,
    pub total_answered: i64,
}

#[derive(Serialize)]
pub struct RecentSession {
    pub id: i64,
    pub started_at: String,
    pub score: i64,
    pub total_questions: i64,
    pub best_streak: i64,
}

#[tauri::command]
pub fn get_overall_stats(db: State<'_, Database>) -> Result<OverallStats, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT COUNT(*), COALESCE(MAX(best_streak), 0), COALESCE(SUM(score), 0), COALESCE(SUM(total_questions), 0) FROM quiz_sessions",
        [],
        |row| Ok(OverallStats {
            total_games: row.get(0)?,
            best_streak: row.get(1)?,
            total_correct: row.get(2)?,
            total_answered: row.get(3)?,
        }),
    ).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_recent_sessions(db: State<'_, Database>, limit: i64) -> Result<Vec<RecentSession>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, started_at, score, total_questions, best_streak FROM quiz_sessions ORDER BY id DESC LIMIT ?1"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![limit], |row| Ok(RecentSession {
        id: row.get(0)?,
        started_at: row.get(1)?,
        score: row.get(2)?,
        total_questions: row.get(3)?,
        best_streak: row.get(4)?,
    })).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}
```

- [ ] **Step 4: Register new commands in lib.rs**

Add `commands::get_overall_stats` and `commands::get_recent_sessions` to the `invoke_handler`.

- [ ] **Step 5: Run tests + build**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test && cargo build
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/ && git commit -m "feat: add stats commands, best_streak migration for frontend"
```

---

### Task 2: TypeScript Types + Theme CSS + App Shell

**Files:**
- Create: `src/types.ts`
- Replace: `src/App.css` — full theme system
- Replace: `src/App.tsx` — screen router with theme wrapper

- [ ] **Step 1: Create types.ts**

`src/types.ts`:

```typescript
export interface Question {
  question_id: number;
  question_type: string;
  question_text: string;
  clue: Clue;
  answers: Answer[];
  time_limit: number | null;
}

export type Clue =
  | { type: "Text"; data: string }
  | { type: "Image"; data: string }
  | { type: "StatBlock"; data: { stats: [string, string][] } }
  | { type: "TextList"; data: string[] }
  | { type: "TwoElements"; data: { element_a: string; element_b: string } };

export interface Answer {
  index: number;
  text: string;
  image: string | null;
}

export interface AnswerResult {
  is_correct: boolean;
  correct_answer_index: number;
  score: number;
  total: number;
  current_streak: number;
  best_streak: number;
}

export interface SessionStats {
  session_id: number;
  score: number;
  total: number;
  current_streak: number;
  best_streak: number;
}

export interface OverallStats {
  total_games: number;
  best_streak: number;
  total_correct: number;
  total_answered: number;
}

export interface RecentSession {
  id: number;
  started_at: string;
  score: number;
  total_questions: number;
  best_streak: number;
}

export interface FetchProgress {
  category: string;
  status: string;
  current: number;
  total: number;
  message: string;
}

export type Screen = "home" | "quiz" | "settings" | "stats";
export type Theme = "duviri" | "grineer" | "lotus";
```

- [ ] **Step 2: Create App.css with full theme system**

`src/App.css` — complete replacement. This is the visual foundation. The subagent should write a comprehensive CSS file with:

1. **Reset + base layout:** `*` reset, `body`/`#root` full height, `.app` flex column centered
2. **Three theme variable blocks** (`[data-theme="duviri"]`, `[data-theme="grineer"]`, `[data-theme="lotus"]`) each setting: `--bg-primary`, `--bg-secondary`, `--bg-card`, `--text-primary`, `--text-secondary`, `--accent`, `--accent-bright`, `--accent-bg`, `--accent-border`, `--button-radius`, `--correct-bg`, `--correct-border`, `--correct-text`, `--wrong-bg`, `--wrong-border`, `--wrong-text`, `--font-heading`, `--font-body`, `--heading-style`, `--heading-weight`, `--heading-spacing`, `--text-transform`, `--divider`
3. **Theme-specific decorations:** Duviri gets `::before`/`::after` bokeh circles. Grineer gets a hazard stripe top bar and grime texture. Lotus gets scanline overlay.
4. **Component classes:** `.logo`, `.play-btn`, `.answer-btn`, `.answer-btn.correct`, `.answer-btn.wrong`, `.answer-btn.dimmed`, `.stat-card`, `.stats-row`, `.top-bar`, `.clue-box`, `.timer-bar`, `.settings-section`, `.theme-card`, `.theme-card.active`, `.next-btn`, `.quit-btn`
5. **All colors/fonts/radii use CSS variables** — never hardcoded

Duviri values: bg `#1a1520`/`#201828`, text `#e8d8c8`, accent `rgba(255,200,150,...)`, Georgia serif italic, pill buttons (20px radius)

Grineer values: bg `#1c1a17`/`#252220`, text `#d4b878`, accent `#c4a135`, Courier New monospace uppercase, angular clip-path buttons, 2px borders

Lotus values: bg `#060e0b`/`#0d2a20`, text `#80ffc0`, accent `#00ff8c`, Segoe UI light weight, thin 1px borders, letter-spacing 1px

- [ ] **Step 3: Create App.tsx with screen router**

`src/App.tsx`:

```tsx
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Theme, Screen, SessionStats, OverallStats } from "./types";
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

  useEffect(() => {
    loadOverallStats();
  }, []);

  useEffect(() => {
    localStorage.setItem("warframedle-theme", theme);
  }, [theme]);

  const timerEnabled = localStorage.getItem("warframedle-timer") === "true";
  const timerSeconds = parseInt(localStorage.getItem("warframedle-timer-seconds") || "15", 10);

  const handlePlay = async () => {
    try {
      await invoke("start_quiz", {
        timer_enabled: timerEnabled,
        timer_seconds: timerSeconds,
      });
      setScreen("quiz");
    } catch (e) {
      console.error("Failed to start quiz:", e);
    }
  };

  const handleQuitQuiz = async () => {
    try {
      await invoke("end_quiz");
    } catch (_) {}
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
        <Stats onBack={() => { loadOverallStats(); setScreen("home"); }} />
      )}
    </div>
  );
}

export default App;
```

- [ ] **Step 4: Create placeholder components** so the build succeeds

Create minimal placeholder files for Home.tsx, Quiz.tsx, Settings.tsx, Stats.tsx, TopBar.tsx — each just returns `<div>Screen Name</div>`. These will be replaced in subsequent tasks.

- [ ] **Step 5: Verify build**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
npm run build
```

- [ ] **Step 6: Commit**

```bash
git add src/ && git commit -m "feat: add theme CSS system, TypeScript types, App shell with screen router"
```

---

### Task 3: TopBar + Home Screen

**Files:**
- Replace: `src/components/TopBar.tsx`
- Replace: `src/components/Home.tsx`

- [ ] **Step 1: Create TopBar.tsx**

Reusable component: back arrow on the left, title centered.

```tsx
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
```

- [ ] **Step 2: Create Home.tsx**

The home screen with: settings pill button (top-right), logo, decorative divider, "Jouer" play button, quick stats row (Parties / Meilleure serie / Precision), stats link.

```tsx
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
      <button className="settings-pill" onClick={onSettings}>
        <span className="gear-icon">&#9881;</span> Paramètres
      </button>

      <div className="logo">Warframedle</div>
      <div className="divider" />

      <button className="play-btn" onClick={onPlay}>Jouer</button>

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
```

- [ ] **Step 3: Verify build + test visually with `npm run tauri dev`**

- [ ] **Step 4: Commit**

```bash
git add src/components/ && git commit -m "feat: add TopBar and Home screen components"
```

---

### Task 4: Quiz Screen (Question + Answers + Feedback + Timer)

**Files:**
- Replace: `src/components/Quiz.tsx`

This is the most complex component. It handles:
1. Fetching questions via `next_question`
2. Rendering clues (switch on `clue.type`)
3. Answer button clicks → `submit_answer`
4. Feedback state (correct/wrong highlighting + "Suivant" button)
5. Timer bar (if enabled) with auto-submit on timeout

- [ ] **Step 1: Create Quiz.tsx**

```tsx
import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Question, AnswerResult, Clue } from "../types";

interface QuizProps {
  timerEnabled: boolean;
  timerSeconds: number;
  onQuit: () => void;
}

export default function Quiz({ timerEnabled, timerSeconds, onQuit }: QuizProps) {
  const [question, setQuestion] = useState<Question | null>(null);
  const [result, setResult] = useState<AnswerResult | null>(null);
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [score, setScore] = useState(0);
  const [total, setTotal] = useState(0);
  const [streak, setStreak] = useState(0);
  const [bestStreak, setBestStreak] = useState(0);
  const [loading, setLoading] = useState(true);
  const startTime = useRef<number>(Date.now());
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const fetchQuestion = async () => {
    setResult(null);
    setSelectedIndex(null);
    setLoading(true);
    try {
      const q = await invoke<Question>("next_question");
      setQuestion(q);
      startTime.current = Date.now();
      if (timerEnabled) {
        if (timerRef.current) clearTimeout(timerRef.current);
        timerRef.current = setTimeout(() => {
          handleAnswer(-1);
        }, timerSeconds * 1000);
      }
    } catch (e) {
      console.error("Failed to get question:", e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchQuestion();
    return () => { if (timerRef.current) clearTimeout(timerRef.current); };
  }, []);

  const handleAnswer = async (answerIndex: number) => {
    if (result) return;
    if (timerRef.current) clearTimeout(timerRef.current);
    const elapsed = (Date.now() - startTime.current) / 1000;
    const idx = answerIndex < 0 ? 0 : answerIndex;
    setSelectedIndex(idx);
    try {
      const res = await invoke<AnswerResult>("submit_answer", {
        answer_index: idx,
        elapsed_seconds: elapsed,
      });
      setResult(res);
      setScore(res.score);
      setTotal(res.total);
      setStreak(res.current_streak);
      setBestStreak(res.best_streak);
    } catch (e) {
      console.error("Failed to submit:", e);
    }
  };

  const handleNext = () => {
    fetchQuestion();
  };

  const getButtonClass = (idx: number): string => {
    if (!result) return "answer-btn";
    if (idx === result.correct_answer_index) return "answer-btn correct";
    if (idx === selectedIndex && !result.is_correct) return "answer-btn wrong";
    return "answer-btn dimmed";
  };

  if (loading && !question) return <div className="quiz"><p className="loading">Chargement...</p></div>;
  if (!question) return null;

  return (
    <div className="quiz">
      <div className="quiz-top">
        <span className="quiz-score">{score}/{total}</span>
        <span className="quiz-streak">
          {streak > 0 ? `${streak} 🔥` : "0"}
        </span>
        <button className="quit-btn" onClick={onQuit}>Quitter</button>
      </div>

      {timerEnabled && !result && (
        <div className="timer-bar-container">
          <div
            className="timer-bar"
            style={{ animationDuration: `${timerSeconds}s` }}
            key={question.question_id}
          />
        </div>
      )}

      <div className="quiz-question">{question.question_text}</div>

      <div className="clue-box">
        {renderClue(question.clue)}
      </div>

      <div className="answers">
        {question.answers.map((a) => (
          <button
            key={a.index}
            className={getButtonClass(a.index)}
            onClick={() => handleAnswer(a.index)}
            disabled={!!result}
          >
            {a.text}
          </button>
        ))}
      </div>

      {result && (
        <button className="next-btn" onClick={handleNext}>
          Suivant
        </button>
      )}
    </div>
  );
}

function renderClue(clue: Clue): JSX.Element {
  switch (clue.type) {
    case "Text":
      return <p className="clue-text">{clue.data}</p>;
    case "TextList":
      return (
        <ul className="clue-list">
          {clue.data.map((item, i) => <li key={i}>{item}</li>)}
        </ul>
      );
    case "Image":
      return <img className="clue-image" src={clue.data} alt="clue" onError={(e) => {
        (e.target as HTMLImageElement).style.display = "none";
      }} />;
    case "StatBlock":
      return (
        <table className="clue-stats">
          <tbody>
            {clue.data.stats.map(([label, value], i) => (
              <tr key={i}><td>{label}</td><td>{value}</td></tr>
            ))}
          </tbody>
        </table>
      );
    case "TwoElements":
      return (
        <div className="clue-elements">
          <span className="element-pill">{clue.data.element_a}</span>
          <span className="element-plus">+</span>
          <span className="element-pill">{clue.data.element_b}</span>
          <span className="element-equals">= ?</span>
        </div>
      );
  }
}
```

- [ ] **Step 2: Add Quiz CSS classes to App.css**

Ensure App.css includes styles for: `.quiz`, `.quiz-top`, `.quiz-score`, `.quiz-streak`, `.quiz-question`, `.clue-box`, `.clue-text`, `.clue-list`, `.clue-image`, `.clue-stats`, `.clue-elements`, `.element-pill`, `.element-plus`, `.element-equals`, `.answers`, `.answer-btn`, `.answer-btn.correct`, `.answer-btn.wrong`, `.answer-btn.dimmed`, `.next-btn`, `.quit-btn`, `.timer-bar-container`, `.timer-bar`, `.loading`

The timer bar uses a CSS animation:
```css
.timer-bar {
  height: 4px;
  background: var(--accent-bright);
  width: 100%;
  animation: shrink linear forwards;
}
@keyframes shrink {
  from { width: 100%; }
  to { width: 0%; background: var(--wrong-border); }
}
```

- [ ] **Step 3: Verify build + test visually**

- [ ] **Step 4: Commit**

```bash
git add src/ && git commit -m "feat: add Quiz screen with clue rendering, answers, feedback, timer"
```

---

### Task 5: Settings Screen

**Files:**
- Replace: `src/components/Settings.tsx`

- [ ] **Step 1: Create Settings.tsx**

The settings screen with: TopBar (back arrow + "Parametres"), theme picker (3 cards), timer toggle + seconds, language toggle (placeholder), data manager (fetch button + progress).

```tsx
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

  useEffect(() => {
    localStorage.setItem("warframedle-timer", String(timerEnabled));
  }, [timerEnabled]);

  useEffect(() => {
    localStorage.setItem("warframedle-timer-seconds", String(timerSeconds));
  }, [timerSeconds]);

  useEffect(() => {
    const unlisten = listen<FetchProgress>("fetch_progress", (e) => setProgress(e.payload));
    return () => { unlisten.then((f) => f()); };
  }, []);

  const handleFetch = async () => {
    setFetching(true);
    try {
      await invoke("fetch_wiki_data");
    } catch (e) {
      console.error("Fetch failed:", e);
    } finally {
      setFetching(false);
      setProgress(null);
    }
  };

  return (
    <div className="settings-screen">
      <TopBar title="Paramètres" onBack={onBack} />

      <div className="settings-section">
        <h3 className="section-title">Thème</h3>
        <div className="theme-cards">
          {THEMES.map((t) => (
            <button
              key={t.id}
              className={`theme-card ${theme === t.id ? "active" : ""}`}
              onClick={() => onThemeChange(t.id)}
            >
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
          <input
            type="checkbox"
            checked={timerEnabled}
            onChange={(e) => setTimerEnabled(e.target.checked)}
          />
        </label>
        {timerEnabled && (
          <label className="toggle-row">
            <span>Secondes par question</span>
            <input
              type="number"
              min={5}
              max={60}
              value={timerSeconds}
              onChange={(e) => setTimerSeconds(parseInt(e.target.value, 10) || 15)}
              className="timer-input"
            />
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
        {progress && (
          <p className="fetch-progress">{progress.message} ({progress.current}/{progress.total})</p>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Add Settings CSS to App.css**

Classes: `.settings-screen`, `.settings-section`, `.section-title`, `.theme-cards`, `.theme-card`, `.theme-card.active`, `.theme-swatch`, `.theme-swatch-duviri` / `-grineer` / `-lotus`, `.theme-name`, `.theme-desc`, `.toggle-row`, `.timer-input`, `.fetch-btn`, `.fetch-progress`

- [ ] **Step 3: Verify build + test visually**

- [ ] **Step 4: Commit**

```bash
git add src/ && git commit -m "feat: add Settings screen with theme picker, timer, data manager"
```

---

### Task 6: Stats Screen

**Files:**
- Replace: `src/components/Stats.tsx`

- [ ] **Step 1: Create Stats.tsx**

```tsx
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
```

- [ ] **Step 2: Add Stats CSS to App.css**

Classes: `.stats-screen`, `.stats-summary`, `.sessions-list`, `.session-row`, `.session-date`, `.session-score`, `.session-streak`, `.no-data`

- [ ] **Step 3: Verify build + test visually**

- [ ] **Step 4: Commit**

```bash
git add src/ && git commit -m "feat: add Stats screen with overall stats and recent sessions"
```

---

### Task 7: Visual Polish + Full Integration Test

**Files:**
- Modify: `src/App.css` — refine any rough edges
- Possibly adjust any component

- [ ] **Step 1: Run `npm run tauri dev` and test the full flow**

1. App opens with Duviri theme on Home screen
2. Quick stats show (0/0/0% if no games played)
3. Click "Jouer" → Quiz starts, question appears with clue + 4 answers
4. Click an answer → feedback shows (green/red highlighting, score updates)
5. Click "Suivant" → next question loads
6. Click "Quitter" → returns to Home, stats update
7. Settings → change theme → entire app reskins
8. Settings → toggle timer → timer bar appears in quiz
9. Settings → fetch data → progress shown
10. Stats → shows recent sessions

- [ ] **Step 2: Fix any visual or functional issues found during testing**

Common issues to check:
- Theme swatches display correctly in Settings
- Clue rendering works for all 5 types (may need to play several rounds)
- Timer bar animates and auto-submits
- Answer buttons not clickable after answering
- Score/streak display correctly
- Back arrows work on Settings/Stats

- [ ] **Step 3: Run final build verification**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
npm run build && cd src-tauri && cargo build
```

- [ ] **Step 4: Commit any fixes**

```bash
git add -A && git commit -m "fix: visual polish and integration fixes for frontend"
```

---

## Phase 4 Complete

At this point you have:
- 3 distinctive Warframe-themed visual skins (Duviri, Grineer, Lotus) switchable in Settings
- Home screen with quick stats and navigation
- Quiz screen with 5 clue types, 4-answer layout, feedback highlighting, optional timer
- Settings with theme picker, timer config, language placeholder, data fetcher
- Stats with overall performance and recent sessions
- Full game loop: Home → Play → Quiz → Feedback → Next → Quit → Home

**Next phase:** Phase 5 (Packaging & Distribution) — building the portable .exe
