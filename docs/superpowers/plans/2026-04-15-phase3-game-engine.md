# Phase 3: Game Engine — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the quiz engine that generates questions from DB data, manages sessions with scoring/streaks, and exposes Tauri commands for the React frontend.

**Architecture:** The `game` module generates questions by querying the populated SQLite DB, tracks session state in a `Mutex<Option<QuizSession>>`, and validates answers. Each of 9 question types has its own generator file. The game module knows nothing about React — it returns serializable structs that Tauri commands pass through.

**Tech Stack:** Rust, rusqlite (existing), rand 0.8, serde (existing)

> **Phase scope:** Phase 3 of 5. Depends on Phase 2 (DB populated with wiki data). Phase 4 (Frontend UI) comes next.

---

## File Structure

```
src-tauri/src/game/
├── mod.rs              — GameState, QuizSession, session lifecycle methods
├── question_types.rs   — Question, Answer, Clue, AnswerResult, StoredQuestion, SessionStats
├── generators/
│   ├── mod.rs          — generate_question() dispatcher + shuffle utility
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

---

### Task 1: Data Types + rand Dependency

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/game/question_types.rs`
- Modify: `src-tauri/src/game/mod.rs`

- [ ] **Step 1: Add rand to Cargo.toml**

Add to `[dependencies]`:
```toml
rand = "0.8"
```

- [ ] **Step 2: Create question_types.rs**

`src-tauri/src/game/question_types.rs`:

```rust
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Question {
    pub question_id: u64,
    pub question_type: String,
    pub question_text: String,
    pub clue: Clue,
    pub answers: Vec<Answer>,
    pub time_limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum Clue {
    Text(String),
    Image(String),
    StatBlock { stats: Vec<(String, String)> },
    TextList(Vec<String>),
    TwoElements { element_a: String, element_b: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct Answer {
    pub index: usize,
    pub text: String,
    pub image: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnswerResult {
    pub is_correct: bool,
    pub correct_answer_index: usize,
    pub score: u32,
    pub total: u32,
    pub current_streak: u32,
    pub best_streak: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionStats {
    pub session_id: i64,
    pub score: u32,
    pub total: u32,
    pub current_streak: u32,
    pub best_streak: u32,
}

#[derive(Debug, Clone)]
pub struct StoredQuestion {
    pub question_id: u64,
    pub question_type: String,
    pub correct_answer_index: usize,
}
```

- [ ] **Step 3: Update game/mod.rs**

```rust
pub mod question_types;
pub mod generators;

use std::sync::Mutex;
use rusqlite::{params, Connection};
use question_types::*;

pub struct GameState {
    pub session: Mutex<Option<QuizSession>>,
    next_question_id: Mutex<u64>,
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            session: Mutex::new(None),
            next_question_id: Mutex::new(1),
        }
    }

    pub fn next_id(&self) -> u64 {
        let mut id = self.next_question_id.lock().unwrap();
        let current = *id;
        *id += 1;
        current
    }
}

pub struct QuizSession {
    pub id: i64,
    pub score: u32,
    pub total: u32,
    pub current_streak: u32,
    pub best_streak: u32,
    pub timer_enabled: bool,
    pub timer_seconds: u32,
    pub current_question: Option<StoredQuestion>,
}

impl QuizSession {
    pub fn start(conn: &Connection, timer_enabled: bool, timer_seconds: u32) -> Result<Self, String> {
        conn.execute(
            "INSERT INTO quiz_sessions (started_at, mode, score, total_questions)
             VALUES (datetime('now'), 'mixed', 0, 0)",
            [],
        ).map_err(|e| e.to_string())?;
        let id = conn.last_insert_rowid();
        Ok(QuizSession {
            id, score: 0, total: 0, current_streak: 0, best_streak: 0,
            timer_enabled, timer_seconds, current_question: None,
        })
    }

    pub fn submit_answer(
        &mut self,
        conn: &Connection,
        answer_index: usize,
        elapsed_seconds: Option<f64>,
    ) -> Result<AnswerResult, String> {
        let stored = self.current_question.take().ok_or("no question pending")?;

        let timed_out = self.timer_enabled
            && elapsed_seconds.map(|e| e > self.timer_seconds as f64).unwrap_or(false);
        let is_correct = !timed_out && answer_index == stored.correct_answer_index;

        self.total += 1;
        if is_correct {
            self.score += 1;
            self.current_streak += 1;
            if self.current_streak > self.best_streak {
                self.best_streak = self.current_streak;
            }
        } else {
            self.current_streak = 0;
        }

        conn.execute(
            "INSERT INTO quiz_answers (session_id, category, correct_item_id, chosen_item_id, is_correct, answered_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
            params![self.id, stored.question_type, stored.correct_answer_index as i64, answer_index as i64, is_correct],
        ).map_err(|e| e.to_string())?;

        Ok(AnswerResult {
            is_correct,
            correct_answer_index: stored.correct_answer_index,
            score: self.score,
            total: self.total,
            current_streak: self.current_streak,
            best_streak: self.best_streak,
        })
    }

    pub fn stats(&self) -> SessionStats {
        SessionStats {
            session_id: self.id,
            score: self.score,
            total: self.total,
            current_streak: self.current_streak,
            best_streak: self.best_streak,
        }
    }

    pub fn end(&self, conn: &Connection) -> Result<SessionStats, String> {
        conn.execute(
            "UPDATE quiz_sessions SET score = ?1, total_questions = ?2 WHERE id = ?3",
            params![self.score, self.total, self.id],
        ).map_err(|e| e.to_string())?;
        Ok(self.stats())
    }
}
```

- [ ] **Step 4: Create empty generators/mod.rs placeholder**

`src-tauri/src/game/generators/mod.rs`:
```rust
// Generator modules will be added in subsequent tasks
```

- [ ] **Step 5: Verify build**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo build
```

- [ ] **Step 6: Commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add src-tauri/Cargo.toml src-tauri/src/game/
git commit -m "feat: add game engine data types, session management, GameState"
```

---

### Task 2: Generator Dispatcher + Warframe By Abilities

**Files:**
- Modify: `src-tauri/src/game/generators/mod.rs`
- Create: `src-tauri/src/game/generators/warframe_by_abilities.rs`

- [ ] **Step 1: Create generators/mod.rs with dispatcher and shuffle utility**

```rust
pub mod warframe_by_abilities;

use rusqlite::Connection;
use rand::seq::SliceRandom;
use rand::Rng;
use crate::game::question_types::*;

pub fn generate_question(
    conn: &Connection,
    question_id: u64,
    time_limit: Option<u32>,
) -> Result<(Question, StoredQuestion), String> {
    let mut rng = rand::thread_rng();
    let types: Vec<fn(&Connection, u64, Option<u32>) -> Result<(Question, StoredQuestion), String>> = vec![
        warframe_by_abilities::generate,
    ];

    let mut indices: Vec<usize> = (0..types.len()).collect();
    indices.shuffle(&mut rng);

    for i in indices {
        match types[i](conn, question_id, time_limit) {
            Ok(result) => return Ok(result),
            Err(_) => continue,
        }
    }

    Err("could not generate any question".to_string())
}

pub fn shuffle_answers(correct: String, wrongs: Vec<String>) -> (Vec<Answer>, usize) {
    let mut all = vec![correct.clone()];
    all.extend(wrongs);
    let mut rng = rand::thread_rng();
    all.shuffle(&mut rng);
    let correct_index = all.iter().position(|a| *a == correct).unwrap();
    let answers = all.into_iter().enumerate().map(|(i, text)| Answer {
        index: i,
        text,
        image: None,
    }).collect();
    (answers, correct_index)
}
```

- [ ] **Step 2: Create warframe_by_abilities.rs**

```rust
use rusqlite::{params, Connection};
use crate::game::question_types::*;
use super::shuffle_answers;

pub fn generate(
    conn: &Connection,
    question_id: u64,
    time_limit: Option<u32>,
) -> Result<(Question, StoredQuestion), String> {
    // Pick a random warframe that has abilities
    let (wf_id, wf_name, wf_type): (i64, String, String) = conn.query_row(
        "SELECT w.id, w.name, w.type FROM warframes w
         WHERE EXISTS (SELECT 1 FROM abilities a WHERE a.warframe_id = w.id)
         ORDER BY RANDOM() LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|e| format!("no warframe with abilities: {e}"))?;

    // Get its abilities
    let mut stmt = conn.prepare(
        "SELECT name FROM abilities WHERE warframe_id = ?1 ORDER BY slot_index"
    ).map_err(|e| e.to_string())?;
    let ability_names: Vec<String> = stmt.query_map(params![wf_id], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    if ability_names.is_empty() {
        return Err("no abilities found".to_string());
    }

    // Get 3 wrong warframes of same type
    let mut wrongs: Vec<String> = conn.prepare(
        "SELECT name FROM warframes WHERE type = ?1 AND id != ?2 ORDER BY RANDOM() LIMIT 3"
    ).map_err(|e| e.to_string())?
    .query_map(params![wf_type, wf_id], |row| row.get(0))
    .map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    // Fallback: if not enough type-matched, fill from all warframes
    if wrongs.len() < 3 {
        let mut more: Vec<String> = conn.prepare(
            "SELECT name FROM warframes WHERE id != ?1 ORDER BY RANDOM() LIMIT ?2"
        ).map_err(|e| e.to_string())?
        .query_map(params![wf_id, (3 - wrongs.len()) as i64], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .filter(|n| !wrongs.contains(n))
        .collect();
        wrongs.append(&mut more);
    }

    if wrongs.len() < 3 {
        return Err("not enough warframes for wrong answers".to_string());
    }

    let (answers, correct_index) = shuffle_answers(wf_name, wrongs);

    Ok((
        Question {
            question_id,
            question_type: "WarframeByAbilities".to_string(),
            question_text: "À quelle Warframe appartiennent ces capacités ?".to_string(),
            clue: Clue::TextList(ability_names),
            answers,
            time_limit,
        },
        StoredQuestion {
            question_id,
            question_type: "WarframeByAbilities".to_string(),
            correct_answer_index: correct_index,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn setup_warframes_with_abilities(conn: &rusqlite::Connection) {
        for (name, wf_type) in &[("Excalibur", "Warframe"), ("Mag", "Warframe"), ("Volt", "Warframe"), ("Frost", "Warframe")] {
            conn.execute(
                "INSERT INTO warframes (name, type) VALUES (?1, ?2)",
                params![name, wf_type],
            ).unwrap();
        }
        let exc_id: i64 = conn.query_row("SELECT id FROM warframes WHERE name = 'Excalibur'", [], |r| r.get(0)).unwrap();
        for (name, slot) in &[("Slash Dash", 1), ("Radial Blind", 2), ("Radial Javelin", 3), ("Exalted Blade", 4)] {
            conn.execute(
                "INSERT INTO abilities (name, warframe_id, slot_index) VALUES (?1, ?2, ?3)",
                params![name, exc_id, slot],
            ).unwrap();
        }
    }

    #[test]
    fn test_generate_warframe_by_abilities() {
        let conn = test_db();
        setup_warframes_with_abilities(&conn);

        let (question, stored) = generate(&conn, 1, None).unwrap();
        assert_eq!(question.question_type, "WarframeByAbilities");
        assert_eq!(question.answers.len(), 4);
        assert!(stored.correct_answer_index < 4);

        let correct_name = &question.answers[stored.correct_answer_index].text;
        assert_eq!(correct_name, "Excalibur");

        if let Clue::TextList(abilities) = &question.clue {
            assert!(!abilities.is_empty());
        } else {
            panic!("expected TextList clue");
        }
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test game::generators::warframe_by_abilities -- --nocapture
```

Expected: 1 test passes.

- [ ] **Step 4: Commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add src-tauri/src/game/
git commit -m "feat: add question generator dispatcher and warframe-by-abilities generator"
```

---

### Task 3: Warframe By Ability + Warframe By Image + Weapon By Stats

**Files:**
- Create: `src-tauri/src/game/generators/warframe_by_ability.rs`
- Create: `src-tauri/src/game/generators/warframe_by_image.rs`
- Create: `src-tauri/src/game/generators/weapon_by_stats.rs`
- Modify: `src-tauri/src/game/generators/mod.rs` — add modules + register in dispatcher

Each generator follows the same pattern as warframe_by_abilities: query DB for correct item, get wrong answers type-matched, shuffle, return Question + StoredQuestion.

- [ ] **Step 1: Create warframe_by_ability.rs**

Key differences from warframe_by_abilities:
- Picks ONE random ability (with its warframe via JOIN)
- Clue is `Clue::Text(ability_name)` (single ability name)
- Wrong answers are other warframes of same type
- Question text: "À quelle Warframe appartient cette capacité ?"

SQL for correct item:
```sql
SELECT a.name, a.icon_path, w.id, w.name, w.type
FROM abilities a JOIN warframes w ON a.warframe_id = w.id
ORDER BY RANDOM() LIMIT 1
```

Test: insert 4 warframes + 1 ability for the first warframe. Generate question. Verify clue is a Text with the ability name, correct answer is the warframe.

- [ ] **Step 2: Create warframe_by_image.rs**

Key differences:
- Picks a random warframe that has an icon_path
- Clue is `Clue::Image(icon_path)`
- Wrong answers are other warframes of same type
- Question text: "Quelle est cette Warframe ?"

SQL: `SELECT id, name, type, icon_path FROM warframes WHERE icon_path IS NOT NULL AND icon_path != '' ORDER BY RANDOM() LIMIT 1`

Test: insert 4 warframes with icon_path set. Verify clue is an Image.

- [ ] **Step 3: Create weapon_by_stats.rs**

Key differences:
- Picks a random weapon with stats
- Clue is `Clue::StatBlock` with crit_chance, crit_multiplier, status_chance, fire_rate
- Wrong answers are other weapons of same type (Primary/Secondary/Melee)
- Question text: "Quelle arme a ces statistiques ?"

SQL: `SELECT id, name, type, crit_chance, crit_multiplier, status_chance, fire_rate FROM weapons WHERE crit_chance IS NOT NULL ORDER BY RANDOM() LIMIT 1`

Format stats as readable strings: `("Chance critique", "25%")`, `("Cadence de tir", "8.75")`, etc.

Test: insert 4 weapons of type Primary with stats. Verify clue is a StatBlock.

- [ ] **Step 4: Update generators/mod.rs**

Add `pub mod warframe_by_ability;`, `pub mod warframe_by_image;`, `pub mod weapon_by_stats;` and register them in the `types` vector inside `generate_question`.

- [ ] **Step 5: Run tests**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test game::generators -- --nocapture
```

Expected: all generator tests pass.

- [ ] **Step 6: Commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add src-tauri/src/game/
git commit -m "feat: add warframe-by-ability, warframe-by-image, weapon-by-stats generators"
```

---

### Task 4: Mod By Effect + Boss Faction + Planet By Resource

**Files:**
- Create: `src-tauri/src/game/generators/mod_by_effect.rs`
- Create: `src-tauri/src/game/generators/boss_faction.rs`
- Create: `src-tauri/src/game/generators/planet_by_resource.rs`
- Modify: `src-tauri/src/game/generators/mod.rs`

- [ ] **Step 1: Create mod_by_effect.rs**

- Picks a random mod with a non-empty effect_description
- Clue is `Clue::Text(mod_name)` — the player sees the mod name
- Answers are 4 effect_description strings (1 correct + 3 from other mods of same mod_type)
- Question text: "Quel est l'effet de ce mod ?"
- The correct answer text is the effect_description, NOT the mod name

SQL for correct: `SELECT id, name, mod_type, effect_description FROM mods WHERE effect_description != '' ORDER BY RANDOM() LIMIT 1`
SQL for wrongs: `SELECT effect_description FROM mods WHERE mod_type = ?1 AND id != ?2 AND effect_description != '' ORDER BY RANDOM() LIMIT 3`

Note: `shuffle_answers` works on the effect_description strings here, not names.

Test: insert 4 mods of same type with different effect_descriptions.

- [ ] **Step 2: Create boss_faction.rs**

- Picks a random boss
- Clue is `Clue::Text(boss_name)`
- Answers are 4 faction names (1 correct + 3 different factions)
- Question text: "À quelle faction appartient ce boss ?"

SQL for correct: `SELECT id, name, faction FROM bosses ORDER BY RANDOM() LIMIT 1`
SQL for wrongs: `SELECT DISTINCT faction FROM bosses WHERE faction != ?1 ORDER BY RANDOM() LIMIT 3`

Test: insert 4 bosses with different factions.

- [ ] **Step 3: Create planet_by_resource.rs**

- Picks a random resource with its planet (via JOIN)
- Clue is `Clue::Text(resource_name)`
- Answers are 4 planet names (1 correct + 3 other planets)
- Question text: "Sur quelle planète trouve-t-on cette ressource ?"

SQL for correct:
```sql
SELECT pr.resource_name, p.id, p.name
FROM planet_resources pr JOIN planets p ON pr.planet_id = p.id
ORDER BY RANDOM() LIMIT 1
```
SQL for wrongs: `SELECT name FROM planets WHERE id != ?1 ORDER BY RANDOM() LIMIT 3`

Test: insert 4 planets + 1 resource linked to the first.

- [ ] **Step 4: Update generators/mod.rs** — add modules + register in dispatcher

- [ ] **Step 5: Run tests and commit**

```bash
cargo test game::generators -- --nocapture
```

```bash
git add src-tauri/src/game/ && git commit -m "feat: add mod-by-effect, boss-faction, planet-by-resource generators"
```

---

### Task 5: Element Combination + Focus School + Complete Dispatcher

**Files:**
- Create: `src-tauri/src/game/generators/element_combination.rs`
- Create: `src-tauri/src/game/generators/focus_school_by_ability.rs`
- Modify: `src-tauri/src/game/generators/mod.rs` — add final modules, complete dispatcher

- [ ] **Step 1: Create element_combination.rs**

- Picks a random combined element (element_type = 'combined')
- Clue is `Clue::TwoElements { element_a, element_b }` from the component_a/component_b fields
- Answers are 4 combined element names (1 correct + 3 other combined elements)
- Question text: "Quel élément résulte de cette combinaison ?"

SQL for correct: `SELECT id, name, component_a, component_b FROM elements WHERE element_type = 'combined' AND component_a != '' ORDER BY RANDOM() LIMIT 1`
SQL for wrongs: `SELECT name FROM elements WHERE element_type = 'combined' AND id != ?1 ORDER BY RANDOM() LIMIT 3`

Test: insert 4 combined elements with component_a/component_b set.

- [ ] **Step 2: Create focus_school_by_ability.rs**

- Picks a random focus ability with its school (via JOIN)
- Clue is `Clue::Text(ability_name)`
- Answers are 4 school names (1 correct + up to 3 other schools — only 5 exist total)
- Question text: "À quelle école de Focus appartient cette capacité ?"

SQL for correct:
```sql
SELECT fa.name, fs.id, fs.name
FROM focus_abilities fa JOIN focus_schools fs ON fa.school_id = fs.id
ORDER BY RANDOM() LIMIT 1
```
SQL for wrongs: `SELECT name FROM focus_schools WHERE id != ?1 ORDER BY RANDOM() LIMIT 3`

Test: insert 4 focus schools + 1 ability for the first school.

- [ ] **Step 3: Update generators/mod.rs — register all 9 generators**

Final `types` vector in `generate_question`:
```rust
let types: Vec<fn(&Connection, u64, Option<u32>) -> Result<(Question, StoredQuestion), String>> = vec![
    warframe_by_abilities::generate,
    warframe_by_ability::generate,
    warframe_by_image::generate,
    weapon_by_stats::generate,
    mod_by_effect::generate,
    boss_faction::generate,
    planet_by_resource::generate,
    element_combination::generate,
    focus_school_by_ability::generate,
];
```

- [ ] **Step 4: Run tests and commit**

```bash
cargo test game::generators -- --nocapture
```

```bash
git add src-tauri/src/game/ && git commit -m "feat: add element-combination, focus-school generators, complete dispatcher with all 9 types"
```

---

### Task 6: Session Management Tests

**Files:**
- Modify: `src-tauri/src/game/mod.rs` — add test module

- [ ] **Step 1: Write session lifecycle tests**

Add to `src-tauri/src/game/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn setup_quiz_data(conn: &Connection) {
        // Insert enough data for at least one question type to work
        for (name, wf_type) in &[("Excalibur", "Warframe"), ("Mag", "Warframe"), ("Volt", "Warframe"), ("Frost", "Warframe")] {
            conn.execute("INSERT INTO warframes (name, type) VALUES (?1, ?2)", params![name, wf_type]).unwrap();
        }
        let exc_id: i64 = conn.query_row("SELECT id FROM warframes WHERE name = 'Excalibur'", [], |r| r.get(0)).unwrap();
        for (name, slot) in &[("Slash Dash", 1), ("Radial Blind", 2), ("Radial Javelin", 3), ("Exalted Blade", 4)] {
            conn.execute("INSERT INTO abilities (name, warframe_id, slot_index) VALUES (?1, ?2, ?3)", params![name, exc_id, slot]).unwrap();
        }
    }

    #[test]
    fn test_start_and_end_session() {
        let conn = test_db();
        let session = QuizSession::start(&conn, false, 0).unwrap();
        assert!(session.id > 0);
        assert_eq!(session.score, 0);
        assert_eq!(session.total, 0);

        let stats = session.end(&conn).unwrap();
        assert_eq!(stats.score, 0);
        assert_eq!(stats.total, 0);

        let saved_score: i64 = conn.query_row(
            "SELECT score FROM quiz_sessions WHERE id = ?1", params![session.id], |r| r.get(0)
        ).unwrap();
        assert_eq!(saved_score, 0);
    }

    #[test]
    fn test_submit_correct_answer() {
        let conn = test_db();
        let mut session = QuizSession::start(&conn, false, 0).unwrap();
        session.current_question = Some(StoredQuestion {
            question_id: 1,
            question_type: "Test".to_string(),
            correct_answer_index: 2,
        });

        let result = session.submit_answer(&conn, 2, None).unwrap();
        assert!(result.is_correct);
        assert_eq!(result.score, 1);
        assert_eq!(result.total, 1);
        assert_eq!(result.current_streak, 1);
    }

    #[test]
    fn test_submit_wrong_answer() {
        let conn = test_db();
        let mut session = QuizSession::start(&conn, false, 0).unwrap();
        session.current_question = Some(StoredQuestion {
            question_id: 1,
            question_type: "Test".to_string(),
            correct_answer_index: 2,
        });

        let result = session.submit_answer(&conn, 0, None).unwrap();
        assert!(!result.is_correct);
        assert_eq!(result.score, 0);
        assert_eq!(result.total, 1);
        assert_eq!(result.current_streak, 0);
        assert_eq!(result.correct_answer_index, 2);
    }

    #[test]
    fn test_streak_tracking() {
        let conn = test_db();
        let mut session = QuizSession::start(&conn, false, 0).unwrap();

        // 3 correct in a row
        for _ in 0..3 {
            session.current_question = Some(StoredQuestion {
                question_id: 1, question_type: "Test".to_string(), correct_answer_index: 0,
            });
            session.submit_answer(&conn, 0, None).unwrap();
        }
        assert_eq!(session.current_streak, 3);
        assert_eq!(session.best_streak, 3);

        // 1 wrong — streak resets
        session.current_question = Some(StoredQuestion {
            question_id: 2, question_type: "Test".to_string(), correct_answer_index: 0,
        });
        session.submit_answer(&conn, 1, None).unwrap();
        assert_eq!(session.current_streak, 0);
        assert_eq!(session.best_streak, 3);

        // 1 more correct
        session.current_question = Some(StoredQuestion {
            question_id: 3, question_type: "Test".to_string(), correct_answer_index: 0,
        });
        session.submit_answer(&conn, 0, None).unwrap();
        assert_eq!(session.current_streak, 1);
        assert_eq!(session.best_streak, 3);
    }

    #[test]
    fn test_timer_enforcement() {
        let conn = test_db();
        let mut session = QuizSession::start(&conn, true, 15).unwrap();
        session.current_question = Some(StoredQuestion {
            question_id: 1, question_type: "Test".to_string(), correct_answer_index: 0,
        });

        // Correct answer but too slow
        let result = session.submit_answer(&conn, 0, Some(20.0)).unwrap();
        assert!(!result.is_correct);
    }

    #[test]
    fn test_submit_without_question_fails() {
        let conn = test_db();
        let mut session = QuizSession::start(&conn, false, 0).unwrap();
        let result = session.submit_answer(&conn, 0, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_full_cycle_with_real_generator() {
        let conn = test_db();
        setup_quiz_data(&conn);
        let game_state = GameState::new();

        let mut session = QuizSession::start(&conn, false, 0).unwrap();
        let qid = game_state.next_id();
        let (question, stored) = generators::generate_question(&conn, qid, None).unwrap();

        assert_eq!(question.answers.len(), 4);
        session.current_question = Some(stored);

        let result = session.submit_answer(&conn, question.answers[0].index, None).unwrap();
        assert_eq!(result.total, 1);

        session.end(&conn).unwrap();
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test game -- --nocapture
```

Expected: all session tests + all generator tests pass.

- [ ] **Step 3: Commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add src-tauri/src/game/
git commit -m "feat: add session management tests — lifecycle, streaks, timer"
```

---

### Task 7: Tauri Command Integration

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add game commands to commands/mod.rs**

Read the existing file first. Add the 5 new commands alongside existing `get_db_stats` and `fetch_wiki_data`:

```rust
use crate::game::{GameState, QuizSession};
use crate::game::question_types::*;
use crate::game::generators;

#[tauri::command]
pub fn start_quiz(
    db: State<'_, Database>,
    game: State<'_, GameState>,
    timer_enabled: bool,
    timer_seconds: u32,
) -> Result<SessionStats, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut session_lock = game.session.lock().map_err(|e| e.to_string())?;

    // End previous session if exists
    if let Some(prev) = session_lock.take() {
        let _ = prev.end(&conn);
    }

    let session = QuizSession::start(&conn, timer_enabled, timer_seconds)?;
    let stats = session.stats();
    *session_lock = Some(session);
    Ok(stats)
}

#[tauri::command]
pub fn next_question(
    db: State<'_, Database>,
    game: State<'_, GameState>,
) -> Result<Question, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut session_lock = game.session.lock().map_err(|e| e.to_string())?;
    let session = session_lock.as_mut().ok_or("no active session")?;

    let qid = game.next_id();
    let time_limit = if session.timer_enabled { Some(session.timer_seconds) } else { None };
    let (question, stored) = generators::generate_question(&conn, qid, time_limit)?;
    session.current_question = Some(stored);
    Ok(question)
}

#[tauri::command]
pub fn submit_answer(
    db: State<'_, Database>,
    game: State<'_, GameState>,
    answer_index: usize,
    elapsed_seconds: Option<f64>,
) -> Result<AnswerResult, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut session_lock = game.session.lock().map_err(|e| e.to_string())?;
    let session = session_lock.as_mut().ok_or("no active session")?;
    session.submit_answer(&conn, answer_index, elapsed_seconds)
}

#[tauri::command]
pub fn get_session_stats(
    game: State<'_, GameState>,
) -> Result<SessionStats, String> {
    let session_lock = game.session.lock().map_err(|e| e.to_string())?;
    let session = session_lock.as_ref().ok_or("no active session")?;
    Ok(session.stats())
}

#[tauri::command]
pub fn end_quiz(
    db: State<'_, Database>,
    game: State<'_, GameState>,
) -> Result<SessionStats, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut session_lock = game.session.lock().map_err(|e| e.to_string())?;
    let session = session_lock.take().ok_or("no active session")?;
    session.end(&conn)
}
```

- [ ] **Step 2: Update lib.rs — register commands + manage GameState**

Add `use crate::game::GameState;` and register the new commands:

```rust
.setup(|app| {
    // ... existing DB setup ...
    app.manage(GameState::new());
    Ok(())
})
.invoke_handler(tauri::generate_handler![
    commands::get_db_stats,
    commands::fetch_wiki_data,
    commands::start_quiz,
    commands::next_question,
    commands::submit_answer,
    commands::get_session_stats,
    commands::end_quiz,
])
```

- [ ] **Step 3: Verify build**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo build
```

- [ ] **Step 4: Run full test suite**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat: wire game engine Tauri commands — start, question, answer, stats, end"
```

---

## Phase 3 Complete

At this point you have:
- 9 question type generators, each tested
- Session management with scoring, streak tracking, and timer enforcement
- Full Tauri command API: start_quiz, next_question, submit_answer, get_session_stats, end_quiz
- All game logic in Rust, React-agnostic

**Next phase:** Phase 4 (Frontend UI) will build the React screens that call these commands.
