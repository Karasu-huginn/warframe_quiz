# Phase 3: Game Engine — Design Spec

## Overview

Build the `game` module that generates quiz questions from the populated database, manages quiz sessions, validates answers, and tracks scoring. Pure Rust backend logic — no UI concerns. React calls Tauri commands and renders what comes back.

## Game Rules

- **Endless mode:** No fixed question count. Player plays until they quit.
- **Mixed categories:** Questions come from all categories randomly (uniform distribution across 9 question types).
- **Type-matched wrong answers:** If the correct answer is an Archwing, wrong answers are also Archwings. Fall back to the full table if fewer than 4 items of that subtype exist.
- **Scoring:** Track total correct, total answered, current streak, and best streak. Streak resets on wrong answer.
- **Optional timer:** Player can enable a per-question time limit (e.g., 15 seconds). If time runs out, the answer is wrong.
- **Feedback:** On answer, reveal whether correct/wrong and highlight the correct answer. No extra info.
- **French UI text:** All question strings are in French.

## Core Flow

```
React calls "start_quiz" → Rust creates a QuizSession
React calls "next_question" → Rust picks random type, generates Question, returns it
React calls "submit_answer" → Rust checks correctness, updates stats, returns AnswerResult
React calls "get_session_stats" → Rust returns current score/streak
React calls "end_quiz" → Rust saves session to DB, returns final stats
```

Rust owns all game state. React holds only the session_id.

## Question Types

9 question types, each generating a clue + 4 answer choices:

| # | Type ID | Question (French) | Clue | Answer choices | DB tables |
|---|---------|-------------------|------|----------------|-----------|
| 1 | WarframeByAbilities | "À quelle Warframe appartiennent ces capacités ?" | 4 ability names | 4 warframe names | warframes + abilities |
| 2 | WarframeByAbility | "À quelle Warframe appartient cette capacité ?" | 1 ability name + icon | 4 warframe names | abilities → warframes |
| 3 | WarframeByImage | "Quelle est cette Warframe ?" | Warframe image | 4 warframe names | warframes |
| 4 | WeaponByStats | "Quelle arme a ces statistiques ?" | Crit/status/fire rate block | 4 weapon names | weapons |
| 5 | ModByEffect | "Quel est l'effet de ce mod ?" | Mod name | 4 effect descriptions | mods |
| 6 | BossFaction | "À quelle faction appartient ce boss ?" | Boss name | 4 faction names | bosses |
| 7 | PlanetByResource | "Sur quelle planète trouve-t-on cette ressource ?" | Resource name | 4 planet names | planet_resources → planets |
| 8 | ElementCombination | "Quel élément résulte de cette combinaison ?" | Two base element names | 4 combined element names | elements |
| 9 | FocusSchoolByAbility | "À quelle école de Focus appartient cette capacité ?" | Focus ability name | 4 school names | focus_abilities → focus_schools |

## Data Structures

### Question (returned to React)

```rust
struct Question {
    question_id: u64,
    question_type: String,
    question_text: String,
    clue: Clue,
    answers: Vec<Answer>,
    time_limit: Option<u32>,
}

enum Clue {
    Text(String),
    Image(String),                  // asset path
    StatBlock { stats: Vec<(String, String)> },  // label-value pairs
    TextList(Vec<String>),          // e.g., 4 ability names
    TwoElements { element_a: String, element_b: String },
}

struct Answer {
    index: usize,
    text: String,
    image: Option<String>,
}
```

### AnswerResult (returned on submit)

```rust
struct AnswerResult {
    is_correct: bool,
    correct_answer_index: usize,
    score: u32,
    total: u32,
    current_streak: u32,
    best_streak: u32,
}
```

### QuizSession (Rust-side state)

```rust
struct QuizSession {
    id: i64,
    score: u32,
    total: u32,
    current_streak: u32,
    best_streak: u32,
    timer_enabled: bool,
    timer_seconds: u32,
    current_question: Option<StoredQuestion>,
    started_at: String,
}

struct StoredQuestion {
    question_id: u64,
    correct_answer_index: usize,
}
```

The session lives in `Mutex<Option<QuizSession>>` in Tauri managed state. Only one session at a time. Starting a new quiz ends the previous one.

## Question Generation Logic

Each question type has a generator function following the same pattern:

1. Query DB for a random "correct" item
2. Extract the clue from that item
3. Query for 3 wrong answers, type-matched (same subtype as correct item)
4. If fewer than 4 items of that subtype exist, fall back to the full table
5. Shuffle all 4 answers, track correct index
6. Return Question struct

**Random type selection:** `next_question` picks a random question type from the 9 available. If a type can't generate a question (not enough data), it picks another type. After 9 failures (all types exhausted), returns an error.

## Wrong Answer Generation

Type-matching rules per question type:

| Question type | Correct item | Wrong answers match on |
|---------------|-------------|----------------------|
| WarframeByAbilities | Warframe (type=X) | Same warframe type (Warframe/Archwing/Necramech) |
| WarframeByAbility | Warframe (type=X) | Same warframe type |
| WarframeByImage | Warframe (type=X) | Same warframe type |
| WeaponByStats | Weapon (type=X) | Same weapon type (Primary/Secondary/Melee) |
| ModByEffect | Mod (mod_type=X) | Same mod type (Rifle/Warframe/Melee/etc.) |
| BossFaction | Boss (faction=X) | Other factions (not the same one) |
| PlanetByResource | Planet | Any other planets |
| ElementCombination | Element (combined) | Other combined elements |
| FocusSchoolByAbility | Focus school | Other schools (only 5 total, always enough) |

## Session Lifecycle

### start_quiz(timer_enabled: bool, timer_seconds: u32) → session_id

1. If a session already exists, end it (save to DB)
2. Insert row into `quiz_sessions` (started_at, mode="mixed")
3. Create QuizSession in memory
4. Return session_id

### next_question(session_id: i64) → Question

1. Verify session_id matches active session
2. Pick random question type
3. Call that type's generator with the DB connection
4. Store correct_answer_index in session state
5. Return Question to React

### submit_answer(session_id: i64, answer_index: usize, elapsed_seconds: Option<f64>) → AnswerResult

1. Verify session_id and that a question is pending
2. Check timer: if enabled and elapsed > limit, treat as wrong
3. Compare answer_index to stored correct_answer_index
4. Update score, total, streak, best_streak
5. Insert row into `quiz_answers`
6. Clear current_question
7. Return AnswerResult

### get_session_stats(session_id: i64) → SessionStats

Returns current score, total, current_streak, best_streak.

### end_quiz(session_id: i64) → FinalStats

1. Update `quiz_sessions` row with final score and total
2. Clear session from memory
3. Return final stats

## Tauri Commands

New commands added to `commands/mod.rs`:

- `start_quiz(timer_enabled, timer_seconds)` → `{ session_id }`
- `next_question(session_id)` → `Question`
- `submit_answer(session_id, answer_index, elapsed_seconds)` → `AnswerResult`
- `get_session_stats(session_id)` → `SessionStats`
- `end_quiz(session_id)` → `FinalStats`

Session state: `Mutex<Option<QuizSession>>` managed alongside the existing `Database` state.

## File Structure

```
src-tauri/src/game/
├── mod.rs              — QuizSession, session management (start/end/stats)
├── question_types.rs   — Question, Answer, Clue, AnswerResult, StoredQuestion structs
├── generators/
│   ├── mod.rs          — generate_question() dispatcher, picks random type
│   ├── warframe_by_abilities.rs
│   ├── warframe_by_ability.rs
│   ├── warframe_by_image.rs
│   ├── weapon_by_stats.rs
│   ├── mod_by_effect.rs
│   ├── boss_faction.rs
│   ├── planet_by_resource.rs
│   ├── element_combination.rs
│   └── focus_school_by_ability.rs
```

## Testing

Each generator is tested with an in-memory DB populated with sample data:
- Insert enough items to generate a question (4+ of the matching subtype)
- Call the generator
- Verify: 4 answers returned, exactly 1 is correct, wrong answers are type-matched, clue matches the correct item

Session management tested:
- Start → next_question → submit_answer cycle
- Streak tracking (correct, correct, wrong → streak resets)
- Timer enforcement (elapsed > limit → wrong)
- end_quiz saves to DB
