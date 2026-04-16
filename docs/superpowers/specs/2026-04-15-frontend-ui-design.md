# Phase 4: Frontend UI — Design Spec

## Overview

Build the React frontend for the Warframedle quiz game. 5 screens with single-page flow (no router). 3 switchable Warframe-themed visual skins. Connects to the existing Rust backend via Tauri IPC commands.

## Screens & Navigation

Single-page flow — React state (`currentScreen`) switches between components. No router library.

```
Home → Quiz → (feedback loop: answer → Next → next question) → Quit → Home
Home → Settings → (back arrow) → Home
Home → Stats → (back arrow) → Home
```

| Screen | Purpose |
|--------|---------|
| Home | Logo, "Jouer" button, quick stats (games played, best streak, accuracy), settings pill button, stats link |
| Quiz | Score/streak bar + quit button at top, question text, clue (varies by type), 4 stacked answer buttons |
| Feedback | Same layout as Quiz but: correct answer green, wrong answer red, buttons disabled, "Suivant" button appears |
| Settings | Theme picker (3 themes), timer toggle + seconds, language toggle (FR/EN placeholder), data manager (fetch + progress) |
| Stats | Historical stats from quiz_sessions: total games, best streak, accuracy %, recent session list |

## Theme System

3 themes, switchable from Settings. Choice saved in localStorage.

### Duviri Painterly (default)
- Background: warm charcoal gradient (`#1a1520` → `#201828`)
- Text: muted cream (`#e0cbb0` / `#e8d8c8`)
- Accent: amber (`rgba(255,200,150,...)`)
- Typography: serif/italic headers (Georgia), normal body
- Buttons: pill-shaped (border-radius: 20px+), thin warm borders
- Decorations: soft bokeh circles (radial gradients), thin gold divider lines
- Error: muted rose (`#e0a0a0` border, `rgba(180,60,60,0.1)` bg)

### Grineer Industrial
- Background: dark steel (`#1c1a17`)
- Text: amber/rust (`#c4a135` headers, `#d4b878` body)
- Accent: rusty gold
- Typography: monospace (`Courier New`), bold uppercase, wide letter-spacing
- Buttons: angled clip-path cuts (`polygon(4px 0,100% 0,calc(100% - 4px) 100%,0 100%)`), thick borders
- Decorations: hazard stripe bar at top (repeating-linear-gradient), grime texture lines
- Error: dark red (`#8b3030` border, `#3a1c1c` bg)

### Lotus Transmission
- Background: deep green-black (`#060e0b` → `#0d2a20`)
- Text: mint/jade (`#00ff8c` headers, `#80ffc0` body)
- Accent: bright mint with glow
- Typography: light weight, wide letter-spacing, small-caps labels
- Buttons: thin mint borders, barely-there background, 1px radius
- Decorations: scanline overlay (repeating-linear-gradient 2px), radial jade glow, "// incoming transmission" style labels
- Error: faded red (`rgba(255,80,80,0.3)` border)

### CSS Implementation

A `data-theme` attribute on the app root div. Each theme sets CSS custom properties:

```css
[data-theme="duviri"] {
  --bg-primary: #1a1520;
  --bg-secondary: #201828;
  --text-primary: #e8d8c8;
  --text-secondary: rgba(224,203,176,0.5);
  --accent: rgba(255,200,150,0.4);
  --accent-bg: rgba(255,220,180,0.08);
  --accent-border: rgba(255,200,150,0.3);
  --button-radius: 20px;
  --correct-bg: rgba(80,180,80,0.15);
  --correct-border: rgba(80,180,80,0.5);
  --wrong-bg: rgba(180,60,60,0.1);
  --wrong-border: rgba(255,100,100,0.3);
  --font-heading: Georgia, 'Times New Roman', serif;
  --font-body: Georgia, 'Times New Roman', serif;
  --heading-style: italic;
  --letter-spacing: 2px;
}
```

Similar variable sets for `grineer` and `lotus`. Components use only CSS variables, never hardcoded colors.

## Home Screen

- Logo: "Warframedle" in theme heading font, large, centered
- Decorative divider below logo
- "Jouer" button: large, pill-shaped (theme-dependent), centered
- Quick stats row: 3 columns (Parties / Meilleure serie / Precision) with numbers and labels
- "Voir les statistiques →" link below stats
- Settings: bordered pill button with gear icon + "Parametres" text, top-right corner
- Stats come from a new Tauri command `get_overall_stats` (or computed from `get_session_stats` calls)

## Quiz Screen

### Layout (top to bottom)
1. **Top bar:** Score (X/Y), streak with fire icon, timer bar (if enabled), Quit button (top-right)
2. **Question text:** centered, theme heading font
3. **Clue area:** rendered based on clue type (see below)
4. **Answer buttons:** 4 full-width stacked buttons with spacing

### Clue Rendering

| Clue type | Component | Display |
|-----------|-----------|---------|
| Text | TextClue | Plain text in a styled box |
| TextList | TextListClue | Bulleted list of ability names |
| Image | ImageClue | Image displayed (with fallback to text if file missing) |
| StatBlock | StatBlockClue | Label-value pairs in a compact table |
| TwoElements | TwoElementsClue | Two element pills with "+" between and "= ?" after |

### Timer (optional)
- Horizontal bar below the top bar
- Shrinks from full width to 0 over `timer_seconds`
- Color transitions from accent to warning to error
- When it hits 0, auto-submits as wrong

## Feedback State

After the player clicks an answer:
- Clicked answer: green border/bg if correct, red if wrong
- Correct answer (if different from clicked): highlighted green
- Other buttons: dimmed, non-clickable
- Score/streak update immediately in the top bar
- "Suivant" button appears at the bottom (manual advance)
- Player clicks "Suivant" → calls `next_question`, returns to Quiz state

## Settings Screen

### Layout
- **Top bar:** back arrow + "Parametres" title
- **Theme picker:** 3 cards showing theme previews (small color swatches + name), selected one highlighted
- **Timer section:** toggle switch + number input for seconds (default 15, disabled when toggle off)
- **Language section:** FR/EN toggle (placeholder — only affects future UI strings)
- **Data Manager section:** "Mettre a jour les donnees" button, progress display, last fetch date

Settings saved to localStorage. Timer settings read by `start_quiz` command parameters.

## Stats Screen

### Layout
- **Top bar:** back arrow + "Statistiques" title
- **Summary cards:** Total games, Best streak ever, Overall accuracy %
- **Recent sessions list:** Last 10 sessions with date, score/total, best streak

Stats fetched via a new Tauri command `get_overall_stats` that queries `quiz_sessions` table.

## Component Structure

```
src/
├── App.tsx              — theme wrapper, screen router
├── App.css              — 3 theme variable sets + shared base styles
├── components/
│   ├── Home.tsx
│   ├── Quiz.tsx
│   ├── Feedback.tsx
│   ├── Settings.tsx
│   ├── Stats.tsx
│   ├── TopBar.tsx       — reusable back arrow + title
│   └── clues/
│       ├── TextClue.tsx
│       ├── TextListClue.tsx
│       ├── ImageClue.tsx
│       ├── StatBlockClue.tsx
│       └── TwoElementsClue.tsx
```

## State Management

No external state library. Local React state + props:

- `App.tsx` holds: `currentScreen`, `theme`, `sessionId`, `timerEnabled`, `timerSeconds`
- `Quiz.tsx` holds: current `Question`, `AnswerResult` (for feedback state)
- `Settings.tsx` reads/writes localStorage
- Theme loaded from localStorage on mount, applied as `data-theme` on root div

## Tauri Commands Used

Existing:
- `start_quiz(timer_enabled, timer_seconds)` → `SessionStats`
- `next_question()` → `Question`
- `submit_answer(answer_index, elapsed_seconds)` → `AnswerResult`
- `get_session_stats()` → `SessionStats`
- `end_quiz()` → `SessionStats`
- `get_db_stats()` → `DbStats`
- `fetch_wiki_data()` → `FetchReport` (with progress events)

New (needed):
- `get_overall_stats()` → `{ total_games, best_streak, total_correct, total_answered }` — queries quiz_sessions table for aggregate stats
- `get_recent_sessions(limit)` → `Vec<{ id, started_at, score, total, best_streak }>` — last N sessions

## New Rust Code Needed

Two new Tauri commands in `commands/mod.rs` + supporting query functions:

```rust
#[tauri::command]
pub fn get_overall_stats(db: State<'_, Database>) -> Result<OverallStats, String>

#[tauri::command]  
pub fn get_recent_sessions(db: State<'_, Database>, limit: i64) -> Result<Vec<RecentSession>, String>
```

These query `quiz_sessions` with `SELECT COUNT(*), MAX(score), SUM(score), SUM(total_questions)` and `SELECT * FROM quiz_sessions ORDER BY id DESC LIMIT ?`.
