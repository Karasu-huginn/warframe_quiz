# Phase 1: Project Foundation & Database — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Scaffold the Tauri v2 + React project and build the complete SQLite database layer with all 32 table schemas and Tier 1 query functions.

**Architecture:** Tauri v2 desktop app. Rust backend owns all state and logic. React frontend communicates via Tauri IPC. SQLite (rusqlite with bundled feature) for local persistence. This phase builds the project skeleton and complete data layer.

**Tech Stack:** Tauri v2, Rust 2021 edition, React 18, TypeScript, Vite 6, SQLite (rusqlite 0.32 with bundled feature)

> **Phase scope:** This is Phase 1 of 5. Later phases: Phase 2 (Data Fetcher), Phase 3 (Game Engine), Phase 4 (Frontend UI), Phase 5 (Packaging & Distribution). Each phase gets its own plan.

**Prerequisites (one-time system setup):**
- Node.js v18+ (`node --version`)
- Rust via rustup (`rustc --version`, `cargo --version`). Install: https://rustup.rs/
- Microsoft C++ Build Tools (required for Rust compilation on Windows). Comes with Visual Studio Build Tools.
- WebView2 runtime (pre-installed on Windows 11)

---

## File Structure

```
warframedle/
├── .gitignore
├── package.json
├── vite.config.ts
├── tsconfig.json
├── index.html
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   └── App.css
├── src-tauri/
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── db/
│       │   ├── mod.rs
│       │   ├── connection.rs
│       │   ├── schema.rs
│       │   ├── models.rs
│       │   └── queries/
│       │       ├── mod.rs
│       │       ├── warframes.rs
│       │       ├── abilities.rs
│       │       ├── weapons.rs
│       │       ├── mods.rs
│       │       ├── characters.rs
│       │       └── quotes.rs
│       ├── fetcher/
│       │   └── mod.rs          (placeholder)
│       ├── game/
│       │   └── mod.rs          (placeholder)
│       └── commands/
│           └── mod.rs
│
│  (existing prototype files — kept for reference, not used by app)
├── codetester1.py
├── warframes.txt / abilities.txt / characters.txt
├── old/ / img/ / *.jpg
└── docs/
```

---

### Task 1: Initialize Git & Scaffold Tauri v2 + React Project

**Files:**
- Create: `.gitignore`
- Create: `package.json`, `vite.config.ts`, `tsconfig.json`, `index.html`
- Create: `src/main.tsx`, `src/App.tsx`, `src/App.css`
- Create: `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`
- Create: `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Initialize git repository**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git init
```

- [ ] **Step 2: Create `.gitignore`**

```gitignore
node_modules/
dist/
src-tauri/target/
.DS_Store
Thumbs.db
*.swp
```

- [ ] **Step 3: Create frontend configuration files**

`package.json`:
```json
{
  "name": "warframedle",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2",
    "@types/react": "^18.3.0",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.0",
    "typescript": "^5.6.0",
    "vite": "^6.0.0"
  }
}
```

`vite.config.ts`:
```typescript
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target:
      process.env.TAURI_ENV_PLATFORM === "windows"
        ? "chrome105"
        : "safari13",
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});
```

`tsconfig.json`:
```json
{
  "compilerOptions": {
    "target": "ES2021",
    "useDefineForClassFields": true,
    "lib": ["ES2021", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"]
}
```

`index.html`:
```html
<!doctype html>
<html lang="fr">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Warframedle</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 4: Create React entry point**

`src/main.tsx`:
```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
```

`src/App.tsx`:
```tsx
function App() {
  return (
    <div>
      <h1>Warframedle</h1>
      <p>Loading...</p>
    </div>
  );
}

export default App;
```

`src/App.css`:
```css
:root {
  font-family: Inter, system-ui, sans-serif;
  color: #e0e0e0;
  background-color: #1a1a2e;
}

body {
  margin: 0;
  display: flex;
  justify-content: center;
  min-height: 100vh;
}

h1 {
  text-align: center;
}
```

- [ ] **Step 5: Create Tauri backend scaffold**

`src-tauri/Cargo.toml`:
```toml
[package]
name = "warframedle"
version = "0.1.0"
edition = "2021"

[lib]
name = "warframedle_lib"
crate-type = ["lib", "cdylib", "staticlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

`src-tauri/build.rs`:
```rust
fn main() {
    tauri_build::build()
}
```

`src-tauri/tauri.conf.json`:
```json
{
  "$schema": "https://raw.githubusercontent.com/nicedoc/tauri/v2/crates/tauri-cli/schema.json",
  "productName": "Warframedle",
  "version": "0.1.0",
  "identifier": "com.warframedle.app",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "title": "Warframedle",
    "windows": [
      {
        "title": "Warframedle",
        "width": 1024,
        "height": 768
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": false
  }
}
```

`src-tauri/capabilities/default.json`:
```json
{
  "identifier": "default",
  "description": "Default capabilities",
  "windows": ["main"],
  "permissions": [
    "core:default"
  ]
}
```

`src-tauri/src/main.rs`:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    warframedle_lib::run()
}
```

`src-tauri/src/lib.rs`:
```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 6: Install dependencies and verify builds**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
npm install
npm run build
cd src-tauri && cargo build
```

Expected: both commands succeed with no errors. The frontend builds to `dist/`, the Rust backend compiles successfully.

- [ ] **Step 7: Commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add .gitignore package.json package-lock.json vite.config.ts tsconfig.json index.html src/ src-tauri/
git commit -m "feat: scaffold Tauri v2 + React project"
```

---

### Task 2: Database Module — Connection & Full Schema

**Files:**
- Modify: `src-tauri/Cargo.toml` (add rusqlite)
- Modify: `src-tauri/src/lib.rs` (declare modules)
- Create: `src-tauri/src/db/mod.rs`
- Create: `src-tauri/src/db/connection.rs`
- Create: `src-tauri/src/db/schema.rs`
- Create: `src-tauri/src/fetcher/mod.rs` (placeholder)
- Create: `src-tauri/src/game/mod.rs` (placeholder)
- Create: `src-tauri/src/commands/mod.rs` (placeholder)

- [ ] **Step 1: Write test for schema creation**

Create `src-tauri/src/db/schema.rs` with the test module first (implementation comes in step 3):

```rust
use rusqlite::Connection;

pub fn create_tables(conn: &Connection) -> Result<(), rusqlite::Error> {
    todo!()
}

#[cfg(test)]
pub fn test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    create_tables(&conn).unwrap();
    conn
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creates_all_tables() {
        let conn = test_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name != 'sqlite_sequence'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 32);
    }

    #[test]
    fn test_foreign_keys_enabled() {
        let conn = test_db();
        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }

    #[test]
    fn test_warframes_table_structure() {
        let conn = test_db();
        conn.execute(
            "INSERT INTO warframes (name, type, description, passive, acquisition)
             VALUES ('Excalibur', 'Warframe', 'A balanced fighter', 'Swordsmanship', 'Starter')",
            [],
        )
        .unwrap();
        let name: String = conn
            .query_row("SELECT name FROM warframes WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(name, "Excalibur");
    }

    #[test]
    fn test_abilities_foreign_key() {
        let conn = test_db();
        conn.execute(
            "INSERT INTO warframes (name, type, description, passive, acquisition)
             VALUES ('Nyx', 'Warframe', 'Mind control', 'Telepathy', 'Boss')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO abilities (name, description, warframe_id) VALUES ('Absorb', 'Absorbs damage', 1)",
            [],
        )
        .unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM abilities WHERE warframe_id = 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
```

- [ ] **Step 2: Add rusqlite dependency and create module structure**

Add to `src-tauri/Cargo.toml` under `[dependencies]`:
```toml
rusqlite = { version = "0.32", features = ["bundled"] }
```

Create `src-tauri/src/db/mod.rs`:
```rust
pub mod connection;
pub mod schema;
```

Create `src-tauri/src/db/connection.rs`:
```rust
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Ok(Database {
            conn: Mutex::new(conn),
        })
    }
}
```

Create placeholder modules:
- `src-tauri/src/fetcher/mod.rs`: empty file
- `src-tauri/src/game/mod.rs`: empty file
- `src-tauri/src/commands/mod.rs`: empty file

Update `src-tauri/src/lib.rs`:
```rust
mod db;
mod fetcher;
mod game;
mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Run tests, verify they fail**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test db::schema
```

Expected: test panics with `not yet implemented` from the `todo!()` in `create_tables`.

- [ ] **Step 4: Implement create_tables with all 32 table definitions**

Replace the `create_tables` function body in `src-tauri/src/db/schema.rs`:

```rust
pub fn create_tables(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "
        -- Tier 1: Core game data
        CREATE TABLE IF NOT EXISTS warframes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            type TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            health REAL,
            shields REAL,
            armor REAL,
            energy REAL,
            sprint_speed REAL,
            passive TEXT NOT NULL DEFAULT '',
            mastery_rank INTEGER,
            acquisition TEXT NOT NULL DEFAULT '',
            release_date TEXT,
            prime_variant TEXT,
            is_vaulted INTEGER NOT NULL DEFAULT 0,
            helminth_ability TEXT,
            sex TEXT,
            icon_path TEXT
        );

        CREATE TABLE IF NOT EXISTS abilities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            cost INTEGER,
            description TEXT NOT NULL DEFAULT '',
            icon_path TEXT,
            warframe_id INTEGER NOT NULL REFERENCES warframes(id),
            slot_index INTEGER,
            is_helminth INTEGER NOT NULL DEFAULT 0,
            augment_mod_name TEXT
        );

        CREATE TABLE IF NOT EXISTS weapons (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            type TEXT NOT NULL,
            subtype TEXT NOT NULL DEFAULT '',
            mastery_rank INTEGER,
            damage_total REAL,
            damage_impact REAL,
            damage_puncture REAL,
            damage_slash REAL,
            crit_chance REAL,
            crit_multiplier REAL,
            status_chance REAL,
            fire_rate REAL,
            magazine_size INTEGER,
            reload_time REAL,
            trigger_type TEXT,
            noise_level TEXT,
            riven_disposition REAL,
            acquisition TEXT NOT NULL DEFAULT '',
            variant_type TEXT,
            base_weapon_id INTEGER REFERENCES weapons(id),
            release_date TEXT,
            icon_path TEXT
        );

        CREATE TABLE IF NOT EXISTS mods (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            polarity TEXT,
            rarity TEXT,
            mod_type TEXT,
            max_rank INTEGER,
            base_drain INTEGER,
            effect_description TEXT NOT NULL DEFAULT '',
            set_name TEXT,
            is_exilus INTEGER NOT NULL DEFAULT 0,
            is_augment INTEGER NOT NULL DEFAULT 0,
            augment_warframe_id INTEGER REFERENCES warframes(id),
            icon_path TEXT
        );

        CREATE TABLE IF NOT EXISTS characters (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL DEFAULT '',
            faction TEXT NOT NULL DEFAULT '',
            location TEXT NOT NULL DEFAULT '',
            role TEXT NOT NULL DEFAULT '',
            voice_actor TEXT,
            icon_path TEXT
        );

        CREATE TABLE IF NOT EXISTS quotes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            character_id INTEGER NOT NULL REFERENCES characters(id),
            quote_text TEXT NOT NULL,
            audio_path TEXT,
            context TEXT NOT NULL DEFAULT ''
        );

        -- Tier 2
        CREATE TABLE IF NOT EXISTS bosses (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            planet TEXT NOT NULL DEFAULT '',
            faction TEXT NOT NULL DEFAULT '',
            description TEXT NOT NULL DEFAULT '',
            warframe_drop TEXT,
            mechanics TEXT NOT NULL DEFAULT '',
            icon_path TEXT,
            character_id INTEGER REFERENCES characters(id)
        );

        CREATE TABLE IF NOT EXISTS companions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            class TEXT NOT NULL,
            breed TEXT,
            health REAL,
            shields REAL,
            armor REAL,
            description TEXT NOT NULL DEFAULT '',
            acquisition TEXT NOT NULL DEFAULT '',
            icon_path TEXT
        );

        CREATE TABLE IF NOT EXISTS companion_precepts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            companion_id INTEGER NOT NULL REFERENCES companions(id),
            icon_path TEXT
        );

        CREATE TABLE IF NOT EXISTS quests (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL DEFAULT '',
            prerequisite_quest_id INTEGER REFERENCES quests(id),
            mastery_requirement INTEGER,
            reward_summary TEXT NOT NULL DEFAULT '',
            storyline_summary TEXT NOT NULL DEFAULT '',
            sort_order INTEGER
        );

        CREATE TABLE IF NOT EXISTS planets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            faction TEXT NOT NULL DEFAULT '',
            open_world_name TEXT,
            hub_name TEXT,
            boss_id INTEGER REFERENCES bosses(id),
            tileset TEXT,
            icon_path TEXT
        );

        CREATE TABLE IF NOT EXISTS planet_resources (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            planet_id INTEGER NOT NULL REFERENCES planets(id),
            resource_name TEXT NOT NULL,
            rarity TEXT
        );

        CREATE TABLE IF NOT EXISTS syndicates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL DEFAULT '',
            leader_name TEXT,
            sigil_path TEXT,
            leader_character_id INTEGER REFERENCES characters(id)
        );

        CREATE TABLE IF NOT EXISTS syndicate_relations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            syndicate_id INTEGER NOT NULL REFERENCES syndicates(id),
            related_syndicate_id INTEGER NOT NULL REFERENCES syndicates(id),
            relation_type TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS relics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            era TEXT NOT NULL,
            is_vaulted INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS relic_rewards (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            relic_id INTEGER NOT NULL REFERENCES relics(id),
            item_name TEXT NOT NULL,
            item_type TEXT NOT NULL DEFAULT '',
            rarity TEXT NOT NULL
        );

        -- Tier 3
        CREATE TABLE IF NOT EXISTS elements (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            element_type TEXT NOT NULL,
            status_effect TEXT NOT NULL DEFAULT '',
            component_a TEXT,
            component_b TEXT
        );

        CREATE TABLE IF NOT EXISTS faction_weaknesses (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            faction TEXT NOT NULL,
            armor_type TEXT NOT NULL DEFAULT '',
            weak_element TEXT NOT NULL,
            strong_element TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS arcanes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            trigger_condition TEXT NOT NULL DEFAULT '',
            effect TEXT NOT NULL DEFAULT '',
            max_rank INTEGER,
            source TEXT NOT NULL DEFAULT '',
            equipment_type TEXT NOT NULL DEFAULT '',
            icon_path TEXT
        );

        CREATE TABLE IF NOT EXISTS focus_schools (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL DEFAULT '',
            symbol_path TEXT
        );

        CREATE TABLE IF NOT EXISTS focus_abilities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            school_id INTEGER NOT NULL REFERENCES focus_schools(id),
            is_waybound INTEGER NOT NULL DEFAULT 0,
            is_passive INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS progenitor_elements (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            warframe_id INTEGER NOT NULL REFERENCES warframes(id),
            element TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS requiem_mods (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            symbol_path TEXT
        );

        CREATE TABLE IF NOT EXISTS incarnon_weapons (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            weapon_id INTEGER NOT NULL REFERENCES weapons(id),
            trigger_description TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS incarnon_evolutions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            incarnon_weapon_id INTEGER NOT NULL REFERENCES incarnon_weapons(id),
            tier INTEGER NOT NULL,
            choice_index INTEGER NOT NULL,
            description TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS railjack_intrinsics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tree_name TEXT NOT NULL,
            rank INTEGER NOT NULL,
            description TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS landing_craft (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            air_support_ability TEXT NOT NULL DEFAULT '',
            description TEXT NOT NULL DEFAULT '',
            icon_path TEXT
        );

        CREATE TABLE IF NOT EXISTS cosmetics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            type TEXT NOT NULL,
            warframe_id INTEGER REFERENCES warframes(id),
            acquisition TEXT NOT NULL DEFAULT '',
            icon_path TEXT
        );

        CREATE TABLE IF NOT EXISTS lore_fragments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            type TEXT NOT NULL,
            content TEXT NOT NULL DEFAULT '',
            audio_path TEXT,
            icon_path TEXT
        );

        -- Game session tracking
        CREATE TABLE IF NOT EXISTS quiz_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            started_at TEXT NOT NULL,
            mode TEXT NOT NULL,
            score INTEGER NOT NULL DEFAULT 0,
            total_questions INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS quiz_answers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL REFERENCES quiz_sessions(id),
            category TEXT NOT NULL,
            correct_item_id INTEGER NOT NULL,
            chosen_item_id INTEGER NOT NULL,
            is_correct INTEGER NOT NULL DEFAULT 0,
            answered_at TEXT NOT NULL
        );

        -- Asset cache
        CREATE TABLE IF NOT EXISTS asset_cache (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_url TEXT NOT NULL UNIQUE,
            local_path TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT '',
            fetched_at TEXT NOT NULL
        );
        ",
    )
}
```

- [ ] **Step 5: Run tests, verify they pass**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test db::schema -- --nocapture
```

Expected: all 4 tests pass.

- [ ] **Step 6: Commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add src-tauri/
git commit -m "feat: add database module with connection and full 32-table schema"
```

---

### Task 3: Tier 1 Models & Queries — Warframes, Abilities, Weapons

**Files:**
- Create: `src-tauri/src/db/models.rs`
- Create: `src-tauri/src/db/queries/mod.rs`
- Create: `src-tauri/src/db/queries/warframes.rs`
- Create: `src-tauri/src/db/queries/abilities.rs`
- Create: `src-tauri/src/db/queries/weapons.rs`
- Modify: `src-tauri/src/db/mod.rs` (add models and queries modules)

- [ ] **Step 1: Update db/mod.rs to declare new modules**

```rust
pub mod connection;
pub mod schema;
pub mod models;
pub mod queries;
```

- [ ] **Step 2: Create models.rs with all Tier 1 structs**

`src-tauri/src/db/models.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warframe {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub wf_type: String,
    pub description: String,
    pub health: Option<f64>,
    pub shields: Option<f64>,
    pub armor: Option<f64>,
    pub energy: Option<f64>,
    pub sprint_speed: Option<f64>,
    pub passive: String,
    pub mastery_rank: Option<i32>,
    pub acquisition: String,
    pub release_date: Option<String>,
    pub prime_variant: Option<String>,
    pub is_vaulted: bool,
    pub helminth_ability: Option<String>,
    pub sex: Option<String>,
    pub icon_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ability {
    pub id: i64,
    pub name: String,
    pub cost: Option<i32>,
    pub description: String,
    pub icon_path: Option<String>,
    pub warframe_id: i64,
    pub slot_index: Option<i32>,
    pub is_helminth: bool,
    pub augment_mod_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weapon {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub weapon_type: String,
    pub subtype: String,
    pub mastery_rank: Option<i32>,
    pub damage_total: Option<f64>,
    pub damage_impact: Option<f64>,
    pub damage_puncture: Option<f64>,
    pub damage_slash: Option<f64>,
    pub crit_chance: Option<f64>,
    pub crit_multiplier: Option<f64>,
    pub status_chance: Option<f64>,
    pub fire_rate: Option<f64>,
    pub magazine_size: Option<i32>,
    pub reload_time: Option<f64>,
    pub trigger_type: Option<String>,
    pub noise_level: Option<String>,
    pub riven_disposition: Option<f64>,
    pub acquisition: String,
    pub variant_type: Option<String>,
    pub base_weapon_id: Option<i64>,
    pub release_date: Option<String>,
    pub icon_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mod {
    pub id: i64,
    pub name: String,
    pub polarity: Option<String>,
    pub rarity: Option<String>,
    pub mod_type: Option<String>,
    pub max_rank: Option<i32>,
    pub base_drain: Option<i32>,
    pub effect_description: String,
    pub set_name: Option<String>,
    pub is_exilus: bool,
    pub is_augment: bool,
    pub augment_warframe_id: Option<i64>,
    pub icon_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub faction: String,
    pub location: String,
    pub role: String,
    pub voice_actor: Option<String>,
    pub icon_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub id: i64,
    pub character_id: i64,
    pub quote_text: String,
    pub audio_path: Option<String>,
    pub context: String,
}
```

- [ ] **Step 3: Create queries/mod.rs**

`src-tauri/src/db/queries/mod.rs`:
```rust
pub mod warframes;
pub mod abilities;
pub mod weapons;
```

- [ ] **Step 4: Create warframes.rs with query functions and tests**

`src-tauri/src/db/queries/warframes.rs`:
```rust
use rusqlite::{params, Connection, OptionalExtension};
use crate::db::models::Warframe;

const COLS: &str = "id, name, type, description, health, shields, armor, energy, \
    sprint_speed, passive, mastery_rank, acquisition, release_date, \
    prime_variant, is_vaulted, helminth_ability, sex, icon_path";

fn row_to_warframe(row: &rusqlite::Row<'_>) -> Result<Warframe, rusqlite::Error> {
    Ok(Warframe {
        id: row.get(0)?,
        name: row.get(1)?,
        wf_type: row.get(2)?,
        description: row.get(3)?,
        health: row.get(4)?,
        shields: row.get(5)?,
        armor: row.get(6)?,
        energy: row.get(7)?,
        sprint_speed: row.get(8)?,
        passive: row.get(9)?,
        mastery_rank: row.get(10)?,
        acquisition: row.get(11)?,
        release_date: row.get(12)?,
        prime_variant: row.get(13)?,
        is_vaulted: row.get(14)?,
        helminth_ability: row.get(15)?,
        sex: row.get(16)?,
        icon_path: row.get(17)?,
    })
}

pub fn insert_warframe(conn: &Connection, wf: &Warframe) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO warframes (name, type, description, health, shields, armor, energy, \
         sprint_speed, passive, mastery_rank, acquisition, release_date, \
         prime_variant, is_vaulted, helminth_ability, sex, icon_path) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            wf.name, wf.wf_type, wf.description, wf.health, wf.shields, wf.armor,
            wf.energy, wf.sprint_speed, wf.passive, wf.mastery_rank, wf.acquisition,
            wf.release_date, wf.prime_variant, wf.is_vaulted, wf.helminth_ability,
            wf.sex, wf.icon_path
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_warframe_by_id(conn: &Connection, id: i64) -> Result<Option<Warframe>, rusqlite::Error> {
    conn.query_row(
        &format!("SELECT {COLS} FROM warframes WHERE id = ?1"),
        params![id],
        |row| row_to_warframe(row),
    )
    .optional()
}

pub fn get_warframe_count(conn: &Connection) -> Result<i64, rusqlite::Error> {
    conn.query_row("SELECT COUNT(*) FROM warframes", [], |row| row.get(0))
}

pub fn get_random_warframes(
    conn: &Connection,
    count: i64,
    exclude_id: Option<i64>,
    type_filter: Option<&str>,
) -> Result<Vec<Warframe>, rusqlite::Error> {
    let exclude = exclude_id.unwrap_or(-1);
    match type_filter {
        Some(tf) => {
            let sql = format!(
                "SELECT {COLS} FROM warframes WHERE id != ?1 AND type = ?2 ORDER BY RANDOM() LIMIT ?3"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![exclude, tf, count], |row| row_to_warframe(row))?;
            rows.collect()
        }
        None => {
            let sql = format!(
                "SELECT {COLS} FROM warframes WHERE id != ?1 ORDER BY RANDOM() LIMIT ?2"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![exclude, count], |row| row_to_warframe(row))?;
            rows.collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn sample(name: &str, wf_type: &str) -> Warframe {
        Warframe {
            id: 0, name: name.to_string(), wf_type: wf_type.to_string(),
            description: "Test".to_string(), health: Some(100.0), shields: Some(100.0),
            armor: Some(200.0), energy: Some(100.0), sprint_speed: Some(1.0),
            passive: "Test".to_string(), mastery_rank: Some(0), acquisition: "Market".to_string(),
            release_date: None, prime_variant: None, is_vaulted: false,
            helminth_ability: None, sex: Some("Male".to_string()), icon_path: None,
        }
    }

    #[test]
    fn test_insert_and_get() {
        let conn = test_db();
        let id = insert_warframe(&conn, &sample("Excalibur", "Warframe")).unwrap();
        let wf = get_warframe_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(wf.name, "Excalibur");
        assert_eq!(wf.wf_type, "Warframe");
        assert_eq!(wf.health, Some(100.0));
    }

    #[test]
    fn test_get_nonexistent() {
        let conn = test_db();
        assert!(get_warframe_by_id(&conn, 999).unwrap().is_none());
    }

    #[test]
    fn test_count() {
        let conn = test_db();
        assert_eq!(get_warframe_count(&conn).unwrap(), 0);
        insert_warframe(&conn, &sample("Excalibur", "Warframe")).unwrap();
        insert_warframe(&conn, &sample("Mag", "Warframe")).unwrap();
        assert_eq!(get_warframe_count(&conn).unwrap(), 2);
    }

    #[test]
    fn test_random_excludes_id() {
        let conn = test_db();
        let id1 = insert_warframe(&conn, &sample("Excalibur", "Warframe")).unwrap();
        insert_warframe(&conn, &sample("Mag", "Warframe")).unwrap();
        insert_warframe(&conn, &sample("Volt", "Warframe")).unwrap();
        let results = get_random_warframes(&conn, 2, Some(id1), None).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|w| w.id != id1));
    }

    #[test]
    fn test_random_type_filter() {
        let conn = test_db();
        insert_warframe(&conn, &sample("Excalibur", "Warframe")).unwrap();
        insert_warframe(&conn, &sample("Amesha", "Archwing")).unwrap();
        let results = get_random_warframes(&conn, 10, None, Some("Archwing")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Amesha");
    }
}
```

- [ ] **Step 5: Run warframe tests**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test db::queries::warframes -- --nocapture
```

Expected: all 5 tests pass.

- [ ] **Step 6: Create abilities.rs with query functions and tests**

`src-tauri/src/db/queries/abilities.rs`:
```rust
use rusqlite::{params, Connection, OptionalExtension};
use crate::db::models::Ability;

const COLS: &str = "id, name, cost, description, icon_path, warframe_id, \
    slot_index, is_helminth, augment_mod_name";

fn row_to_ability(row: &rusqlite::Row<'_>) -> Result<Ability, rusqlite::Error> {
    Ok(Ability {
        id: row.get(0)?,
        name: row.get(1)?,
        cost: row.get(2)?,
        description: row.get(3)?,
        icon_path: row.get(4)?,
        warframe_id: row.get(5)?,
        slot_index: row.get(6)?,
        is_helminth: row.get(7)?,
        augment_mod_name: row.get(8)?,
    })
}

pub fn insert_ability(conn: &Connection, a: &Ability) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO abilities (name, cost, description, icon_path, warframe_id, \
         slot_index, is_helminth, augment_mod_name) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            a.name, a.cost, a.description, a.icon_path, a.warframe_id,
            a.slot_index, a.is_helminth, a.augment_mod_name
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_ability_by_id(conn: &Connection, id: i64) -> Result<Option<Ability>, rusqlite::Error> {
    conn.query_row(
        &format!("SELECT {COLS} FROM abilities WHERE id = ?1"),
        params![id],
        |row| row_to_ability(row),
    )
    .optional()
}

pub fn get_abilities_by_warframe(conn: &Connection, warframe_id: i64) -> Result<Vec<Ability>, rusqlite::Error> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM abilities WHERE warframe_id = ?1 ORDER BY slot_index"
    ))?;
    let rows = stmt.query_map(params![warframe_id], |row| row_to_ability(row))?;
    rows.collect()
}

pub fn get_ability_count(conn: &Connection) -> Result<i64, rusqlite::Error> {
    conn.query_row("SELECT COUNT(*) FROM abilities", [], |row| row.get(0))
}

pub fn get_random_abilities(
    conn: &Connection,
    count: i64,
    exclude_warframe_id: Option<i64>,
) -> Result<Vec<Ability>, rusqlite::Error> {
    let exclude = exclude_warframe_id.unwrap_or(-1);
    let sql = format!(
        "SELECT {COLS} FROM abilities WHERE warframe_id != ?1 ORDER BY RANDOM() LIMIT ?2"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![exclude, count], |row| row_to_ability(row))?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Warframe;
    use crate::db::queries::warframes::insert_warframe;
    use crate::db::schema::test_db;

    fn setup_warframe(conn: &Connection) -> i64 {
        let wf = Warframe {
            id: 0, name: "Nyx".to_string(), wf_type: "Warframe".to_string(),
            description: "".to_string(), health: None, shields: None, armor: None,
            energy: None, sprint_speed: None, passive: "".to_string(),
            mastery_rank: None, acquisition: "".to_string(), release_date: None,
            prime_variant: None, is_vaulted: false, helminth_ability: None,
            sex: None, icon_path: None,
        };
        insert_warframe(conn, &wf).unwrap()
    }

    fn sample(name: &str, warframe_id: i64, slot: i32) -> Ability {
        Ability {
            id: 0, name: name.to_string(), cost: Some(25),
            description: "Test".to_string(), icon_path: None,
            warframe_id, slot_index: Some(slot), is_helminth: false,
            augment_mod_name: None,
        }
    }

    #[test]
    fn test_insert_and_get() {
        let conn = test_db();
        let wf_id = setup_warframe(&conn);
        let id = insert_ability(&conn, &sample("Absorb", wf_id, 1)).unwrap();
        let a = get_ability_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(a.name, "Absorb");
        assert_eq!(a.warframe_id, wf_id);
    }

    #[test]
    fn test_get_by_warframe() {
        let conn = test_db();
        let wf_id = setup_warframe(&conn);
        insert_ability(&conn, &sample("Absorb", wf_id, 4)).unwrap();
        insert_ability(&conn, &sample("Mind Control", wf_id, 1)).unwrap();
        let abilities = get_abilities_by_warframe(&conn, wf_id).unwrap();
        assert_eq!(abilities.len(), 2);
        assert_eq!(abilities[0].name, "Mind Control"); // ordered by slot_index
    }

    #[test]
    fn test_count() {
        let conn = test_db();
        let wf_id = setup_warframe(&conn);
        assert_eq!(get_ability_count(&conn).unwrap(), 0);
        insert_ability(&conn, &sample("Absorb", wf_id, 1)).unwrap();
        assert_eq!(get_ability_count(&conn).unwrap(), 1);
    }

    #[test]
    fn test_random_excludes_warframe() {
        let conn = test_db();
        let wf1 = setup_warframe(&conn);
        let wf2 = {
            let wf = Warframe {
                id: 0, name: "Mag".to_string(), wf_type: "Warframe".to_string(),
                description: "".to_string(), health: None, shields: None, armor: None,
                energy: None, sprint_speed: None, passive: "".to_string(),
                mastery_rank: None, acquisition: "".to_string(), release_date: None,
                prime_variant: None, is_vaulted: false, helminth_ability: None,
                sex: None, icon_path: None,
            };
            insert_warframe(&conn, &wf).unwrap()
        };
        insert_ability(&conn, &sample("Absorb", wf1, 1)).unwrap();
        insert_ability(&conn, &sample("Pull", wf2, 1)).unwrap();
        let results = get_random_abilities(&conn, 10, Some(wf1)).unwrap();
        assert!(results.iter().all(|a| a.warframe_id != wf1));
    }
}
```

- [ ] **Step 7: Run ability tests**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test db::queries::abilities -- --nocapture
```

Expected: all 4 tests pass.

- [ ] **Step 8: Create weapons.rs with query functions and tests**

`src-tauri/src/db/queries/weapons.rs`:
```rust
use rusqlite::{params, Connection, OptionalExtension};
use crate::db::models::Weapon;

const COLS: &str = "id, name, type, subtype, mastery_rank, damage_total, \
    damage_impact, damage_puncture, damage_slash, crit_chance, crit_multiplier, \
    status_chance, fire_rate, magazine_size, reload_time, trigger_type, \
    noise_level, riven_disposition, acquisition, variant_type, base_weapon_id, \
    release_date, icon_path";

fn row_to_weapon(row: &rusqlite::Row<'_>) -> Result<Weapon, rusqlite::Error> {
    Ok(Weapon {
        id: row.get(0)?,
        name: row.get(1)?,
        weapon_type: row.get(2)?,
        subtype: row.get(3)?,
        mastery_rank: row.get(4)?,
        damage_total: row.get(5)?,
        damage_impact: row.get(6)?,
        damage_puncture: row.get(7)?,
        damage_slash: row.get(8)?,
        crit_chance: row.get(9)?,
        crit_multiplier: row.get(10)?,
        status_chance: row.get(11)?,
        fire_rate: row.get(12)?,
        magazine_size: row.get(13)?,
        reload_time: row.get(14)?,
        trigger_type: row.get(15)?,
        noise_level: row.get(16)?,
        riven_disposition: row.get(17)?,
        acquisition: row.get(18)?,
        variant_type: row.get(19)?,
        base_weapon_id: row.get(20)?,
        release_date: row.get(21)?,
        icon_path: row.get(22)?,
    })
}

pub fn insert_weapon(conn: &Connection, w: &Weapon) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO weapons (name, type, subtype, mastery_rank, damage_total, \
         damage_impact, damage_puncture, damage_slash, crit_chance, crit_multiplier, \
         status_chance, fire_rate, magazine_size, reload_time, trigger_type, \
         noise_level, riven_disposition, acquisition, variant_type, base_weapon_id, \
         release_date, icon_path) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
        params![
            w.name, w.weapon_type, w.subtype, w.mastery_rank, w.damage_total,
            w.damage_impact, w.damage_puncture, w.damage_slash, w.crit_chance,
            w.crit_multiplier, w.status_chance, w.fire_rate, w.magazine_size,
            w.reload_time, w.trigger_type, w.noise_level, w.riven_disposition,
            w.acquisition, w.variant_type, w.base_weapon_id, w.release_date, w.icon_path
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_weapon_by_id(conn: &Connection, id: i64) -> Result<Option<Weapon>, rusqlite::Error> {
    conn.query_row(
        &format!("SELECT {COLS} FROM weapons WHERE id = ?1"),
        params![id],
        |row| row_to_weapon(row),
    )
    .optional()
}

pub fn get_weapon_count(conn: &Connection) -> Result<i64, rusqlite::Error> {
    conn.query_row("SELECT COUNT(*) FROM weapons", [], |row| row.get(0))
}

pub fn get_random_weapons(
    conn: &Connection,
    count: i64,
    exclude_id: Option<i64>,
    type_filter: Option<&str>,
) -> Result<Vec<Weapon>, rusqlite::Error> {
    let exclude = exclude_id.unwrap_or(-1);
    match type_filter {
        Some(tf) => {
            let sql = format!(
                "SELECT {COLS} FROM weapons WHERE id != ?1 AND type = ?2 ORDER BY RANDOM() LIMIT ?3"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![exclude, tf, count], |row| row_to_weapon(row))?;
            rows.collect()
        }
        None => {
            let sql = format!(
                "SELECT {COLS} FROM weapons WHERE id != ?1 ORDER BY RANDOM() LIMIT ?2"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![exclude, count], |row| row_to_weapon(row))?;
            rows.collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn sample(name: &str, weapon_type: &str) -> Weapon {
        Weapon {
            id: 0, name: name.to_string(), weapon_type: weapon_type.to_string(),
            subtype: "Rifle".to_string(), mastery_rank: Some(5),
            damage_total: Some(30.0), damage_impact: Some(6.0),
            damage_puncture: Some(18.0), damage_slash: Some(6.0),
            crit_chance: Some(0.1), crit_multiplier: Some(1.6),
            status_chance: Some(0.14), fire_rate: Some(8.75),
            magazine_size: Some(60), reload_time: Some(2.6),
            trigger_type: Some("Auto".to_string()),
            noise_level: Some("Alarming".to_string()),
            riven_disposition: Some(1.3), acquisition: "Market".to_string(),
            variant_type: None, base_weapon_id: None,
            release_date: None, icon_path: None,
        }
    }

    #[test]
    fn test_insert_and_get() {
        let conn = test_db();
        let id = insert_weapon(&conn, &sample("Boltor", "Primary")).unwrap();
        let w = get_weapon_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(w.name, "Boltor");
        assert_eq!(w.weapon_type, "Primary");
        assert_eq!(w.damage_total, Some(30.0));
    }

    #[test]
    fn test_count() {
        let conn = test_db();
        assert_eq!(get_weapon_count(&conn).unwrap(), 0);
        insert_weapon(&conn, &sample("Boltor", "Primary")).unwrap();
        insert_weapon(&conn, &sample("Lex", "Secondary")).unwrap();
        assert_eq!(get_weapon_count(&conn).unwrap(), 2);
    }

    #[test]
    fn test_random_type_filter() {
        let conn = test_db();
        insert_weapon(&conn, &sample("Boltor", "Primary")).unwrap();
        insert_weapon(&conn, &sample("Lex", "Secondary")).unwrap();
        insert_weapon(&conn, &sample("Skana", "Melee")).unwrap();
        let results = get_random_weapons(&conn, 10, None, Some("Primary")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Boltor");
    }

    #[test]
    fn test_random_excludes_id() {
        let conn = test_db();
        let id1 = insert_weapon(&conn, &sample("Boltor", "Primary")).unwrap();
        insert_weapon(&conn, &sample("Braton", "Primary")).unwrap();
        let results = get_random_weapons(&conn, 10, Some(id1), None).unwrap();
        assert!(results.iter().all(|w| w.id != id1));
    }
}
```

- [ ] **Step 9: Run weapon tests**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test db::queries::weapons -- --nocapture
```

Expected: all 4 tests pass.

- [ ] **Step 10: Run all tests to verify nothing broke**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test
```

Expected: all tests pass (schema tests + warframe tests + ability tests + weapon tests).

- [ ] **Step 11: Commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add src-tauri/src/db/
git commit -m "feat: add Tier 1 models and queries for warframes, abilities, weapons"
```

---

### Task 4: Tier 1 Queries — Mods, Characters, Quotes

**Files:**
- Create: `src-tauri/src/db/queries/mods.rs`
- Create: `src-tauri/src/db/queries/characters.rs`
- Create: `src-tauri/src/db/queries/quotes.rs`
- Modify: `src-tauri/src/db/queries/mod.rs` (add new modules)

- [ ] **Step 1: Update queries/mod.rs**

```rust
pub mod warframes;
pub mod abilities;
pub mod weapons;
pub mod mods;
pub mod characters;
pub mod quotes;
```

- [ ] **Step 2: Create mods.rs with query functions and tests**

`src-tauri/src/db/queries/mods.rs`:
```rust
use rusqlite::{params, Connection, OptionalExtension};
use crate::db::models::Mod;

const COLS: &str = "id, name, polarity, rarity, mod_type, max_rank, base_drain, \
    effect_description, set_name, is_exilus, is_augment, augment_warframe_id, icon_path";

fn row_to_mod(row: &rusqlite::Row<'_>) -> Result<Mod, rusqlite::Error> {
    Ok(Mod {
        id: row.get(0)?,
        name: row.get(1)?,
        polarity: row.get(2)?,
        rarity: row.get(3)?,
        mod_type: row.get(4)?,
        max_rank: row.get(5)?,
        base_drain: row.get(6)?,
        effect_description: row.get(7)?,
        set_name: row.get(8)?,
        is_exilus: row.get(9)?,
        is_augment: row.get(10)?,
        augment_warframe_id: row.get(11)?,
        icon_path: row.get(12)?,
    })
}

pub fn insert_mod(conn: &Connection, m: &Mod) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO mods (name, polarity, rarity, mod_type, max_rank, base_drain, \
         effect_description, set_name, is_exilus, is_augment, augment_warframe_id, icon_path) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            m.name, m.polarity, m.rarity, m.mod_type, m.max_rank, m.base_drain,
            m.effect_description, m.set_name, m.is_exilus, m.is_augment,
            m.augment_warframe_id, m.icon_path
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_mod_by_id(conn: &Connection, id: i64) -> Result<Option<Mod>, rusqlite::Error> {
    conn.query_row(
        &format!("SELECT {COLS} FROM mods WHERE id = ?1"),
        params![id],
        |row| row_to_mod(row),
    )
    .optional()
}

pub fn get_mod_count(conn: &Connection) -> Result<i64, rusqlite::Error> {
    conn.query_row("SELECT COUNT(*) FROM mods", [], |row| row.get(0))
}

pub fn get_random_mods(
    conn: &Connection,
    count: i64,
    exclude_id: Option<i64>,
    mod_type_filter: Option<&str>,
) -> Result<Vec<Mod>, rusqlite::Error> {
    let exclude = exclude_id.unwrap_or(-1);
    match mod_type_filter {
        Some(mt) => {
            let sql = format!(
                "SELECT {COLS} FROM mods WHERE id != ?1 AND mod_type = ?2 ORDER BY RANDOM() LIMIT ?3"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![exclude, mt, count], |row| row_to_mod(row))?;
            rows.collect()
        }
        None => {
            let sql = format!(
                "SELECT {COLS} FROM mods WHERE id != ?1 ORDER BY RANDOM() LIMIT ?2"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![exclude, count], |row| row_to_mod(row))?;
            rows.collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn sample(name: &str, mod_type: &str) -> Mod {
        Mod {
            id: 0, name: name.to_string(), polarity: Some("Madurai".to_string()),
            rarity: Some("Rare".to_string()), mod_type: Some(mod_type.to_string()),
            max_rank: Some(10), base_drain: Some(4),
            effect_description: "+damage".to_string(), set_name: None,
            is_exilus: false, is_augment: false, augment_warframe_id: None,
            icon_path: None,
        }
    }

    #[test]
    fn test_insert_and_get() {
        let conn = test_db();
        let id = insert_mod(&conn, &sample("Serration", "Rifle")).unwrap();
        let m = get_mod_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(m.name, "Serration");
        assert_eq!(m.mod_type, Some("Rifle".to_string()));
    }

    #[test]
    fn test_count() {
        let conn = test_db();
        assert_eq!(get_mod_count(&conn).unwrap(), 0);
        insert_mod(&conn, &sample("Serration", "Rifle")).unwrap();
        assert_eq!(get_mod_count(&conn).unwrap(), 1);
    }

    #[test]
    fn test_random_mod_type_filter() {
        let conn = test_db();
        insert_mod(&conn, &sample("Serration", "Rifle")).unwrap();
        insert_mod(&conn, &sample("Hornet Strike", "Pistol")).unwrap();
        let results = get_random_mods(&conn, 10, None, Some("Rifle")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Serration");
    }
}
```

- [ ] **Step 3: Run mod tests**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test db::queries::mods -- --nocapture
```

Expected: all 3 tests pass.

- [ ] **Step 4: Create characters.rs with query functions and tests**

`src-tauri/src/db/queries/characters.rs`:
```rust
use rusqlite::{params, Connection, OptionalExtension};
use crate::db::models::Character;

const COLS: &str = "id, name, description, faction, location, role, voice_actor, icon_path";

fn row_to_character(row: &rusqlite::Row<'_>) -> Result<Character, rusqlite::Error> {
    Ok(Character {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        faction: row.get(3)?,
        location: row.get(4)?,
        role: row.get(5)?,
        voice_actor: row.get(6)?,
        icon_path: row.get(7)?,
    })
}

pub fn insert_character(conn: &Connection, c: &Character) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO characters (name, description, faction, location, role, voice_actor, icon_path) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![c.name, c.description, c.faction, c.location, c.role, c.voice_actor, c.icon_path],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_character_by_id(conn: &Connection, id: i64) -> Result<Option<Character>, rusqlite::Error> {
    conn.query_row(
        &format!("SELECT {COLS} FROM characters WHERE id = ?1"),
        params![id],
        |row| row_to_character(row),
    )
    .optional()
}

pub fn get_character_count(conn: &Connection) -> Result<i64, rusqlite::Error> {
    conn.query_row("SELECT COUNT(*) FROM characters", [], |row| row.get(0))
}

pub fn get_random_characters(
    conn: &Connection,
    count: i64,
    exclude_id: Option<i64>,
) -> Result<Vec<Character>, rusqlite::Error> {
    let exclude = exclude_id.unwrap_or(-1);
    let sql = format!(
        "SELECT {COLS} FROM characters WHERE id != ?1 ORDER BY RANDOM() LIMIT ?2"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![exclude, count], |row| row_to_character(row))?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;

    fn sample(name: &str, faction: &str) -> Character {
        Character {
            id: 0, name: name.to_string(), description: "Test".to_string(),
            faction: faction.to_string(), location: "Relay".to_string(),
            role: "Vendor".to_string(), voice_actor: None, icon_path: None,
        }
    }

    #[test]
    fn test_insert_and_get() {
        let conn = test_db();
        let id = insert_character(&conn, &sample("Lotus", "Tenno")).unwrap();
        let c = get_character_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(c.name, "Lotus");
        assert_eq!(c.faction, "Tenno");
    }

    #[test]
    fn test_count() {
        let conn = test_db();
        assert_eq!(get_character_count(&conn).unwrap(), 0);
        insert_character(&conn, &sample("Lotus", "Tenno")).unwrap();
        insert_character(&conn, &sample("Ordis", "Cephalon")).unwrap();
        assert_eq!(get_character_count(&conn).unwrap(), 2);
    }

    #[test]
    fn test_random_excludes_id() {
        let conn = test_db();
        let id1 = insert_character(&conn, &sample("Lotus", "Tenno")).unwrap();
        insert_character(&conn, &sample("Ordis", "Cephalon")).unwrap();
        insert_character(&conn, &sample("Teshin", "Tenno")).unwrap();
        let results = get_random_characters(&conn, 2, Some(id1)).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|c| c.id != id1));
    }
}
```

- [ ] **Step 5: Run character tests**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test db::queries::characters -- --nocapture
```

Expected: all 3 tests pass.

- [ ] **Step 6: Create quotes.rs with query functions and tests**

`src-tauri/src/db/queries/quotes.rs`:
```rust
use rusqlite::{params, Connection, OptionalExtension};
use crate::db::models::Quote;

const COLS: &str = "id, character_id, quote_text, audio_path, context";

fn row_to_quote(row: &rusqlite::Row<'_>) -> Result<Quote, rusqlite::Error> {
    Ok(Quote {
        id: row.get(0)?,
        character_id: row.get(1)?,
        quote_text: row.get(2)?,
        audio_path: row.get(3)?,
        context: row.get(4)?,
    })
}

pub fn insert_quote(conn: &Connection, q: &Quote) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO quotes (character_id, quote_text, audio_path, context) \
         VALUES (?1, ?2, ?3, ?4)",
        params![q.character_id, q.quote_text, q.audio_path, q.context],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_quote_by_id(conn: &Connection, id: i64) -> Result<Option<Quote>, rusqlite::Error> {
    conn.query_row(
        &format!("SELECT {COLS} FROM quotes WHERE id = ?1"),
        params![id],
        |row| row_to_quote(row),
    )
    .optional()
}

pub fn get_quotes_by_character(conn: &Connection, character_id: i64) -> Result<Vec<Quote>, rusqlite::Error> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM quotes WHERE character_id = ?1"
    ))?;
    let rows = stmt.query_map(params![character_id], |row| row_to_quote(row))?;
    rows.collect()
}

pub fn get_quote_count(conn: &Connection) -> Result<i64, rusqlite::Error> {
    conn.query_row("SELECT COUNT(*) FROM quotes", [], |row| row.get(0))
}

pub fn get_random_quote(conn: &Connection) -> Result<Option<Quote>, rusqlite::Error> {
    conn.query_row(
        &format!("SELECT {COLS} FROM quotes ORDER BY RANDOM() LIMIT 1"),
        [],
        |row| row_to_quote(row),
    )
    .optional()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Character;
    use crate::db::queries::characters::insert_character;
    use crate::db::schema::test_db;

    fn setup_character(conn: &Connection, name: &str) -> i64 {
        let c = Character {
            id: 0, name: name.to_string(), description: "".to_string(),
            faction: "Tenno".to_string(), location: "".to_string(),
            role: "".to_string(), voice_actor: None, icon_path: None,
        };
        insert_character(conn, &c).unwrap()
    }

    fn sample(character_id: i64, text: &str) -> Quote {
        Quote {
            id: 0, character_id, quote_text: text.to_string(),
            audio_path: None, context: "greeting".to_string(),
        }
    }

    #[test]
    fn test_insert_and_get() {
        let conn = test_db();
        let char_id = setup_character(&conn, "Lotus");
        let id = insert_quote(&conn, &sample(char_id, "Dream, Tenno.")).unwrap();
        let q = get_quote_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(q.quote_text, "Dream, Tenno.");
        assert_eq!(q.character_id, char_id);
    }

    #[test]
    fn test_get_by_character() {
        let conn = test_db();
        let char_id = setup_character(&conn, "Lotus");
        insert_quote(&conn, &sample(char_id, "Dream, Tenno.")).unwrap();
        insert_quote(&conn, &sample(char_id, "Focus, Tenno.")).unwrap();
        let quotes = get_quotes_by_character(&conn, char_id).unwrap();
        assert_eq!(quotes.len(), 2);
    }

    #[test]
    fn test_count_and_random() {
        let conn = test_db();
        let char_id = setup_character(&conn, "Lotus");
        assert_eq!(get_quote_count(&conn).unwrap(), 0);
        assert!(get_random_quote(&conn).unwrap().is_none());

        insert_quote(&conn, &sample(char_id, "Dream, Tenno.")).unwrap();
        assert_eq!(get_quote_count(&conn).unwrap(), 1);
        let q = get_random_quote(&conn).unwrap().unwrap();
        assert_eq!(q.quote_text, "Dream, Tenno.");
    }
}
```

- [ ] **Step 7: Run quote tests**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test db::queries::quotes -- --nocapture
```

Expected: all 3 tests pass.

- [ ] **Step 8: Run full test suite**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test
```

Expected: all tests pass (schema + warframes + abilities + weapons + mods + characters + quotes).

- [ ] **Step 9: Commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add src-tauri/src/db/
git commit -m "feat: add Tier 1 queries for mods, characters, quotes"
```

---

### Task 5: Tauri Commands & IPC Verification

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/App.tsx`

- [ ] **Step 1: Write the Tauri command**

`src-tauri/src/commands/mod.rs`:
```rust
use serde::Serialize;
use tauri::State;
use crate::db::connection::Database;
use crate::db::queries::{warframes, abilities, weapons, mods, characters, quotes};

#[derive(Serialize)]
pub struct DbStats {
    pub warframe_count: i64,
    pub ability_count: i64,
    pub weapon_count: i64,
    pub mod_count: i64,
    pub character_count: i64,
    pub quote_count: i64,
}

#[tauri::command]
pub fn get_db_stats(db: State<'_, Database>) -> Result<DbStats, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(DbStats {
        warframe_count: warframes::get_warframe_count(&conn).map_err(|e| e.to_string())?,
        ability_count: abilities::get_ability_count(&conn).map_err(|e| e.to_string())?,
        weapon_count: weapons::get_weapon_count(&conn).map_err(|e| e.to_string())?,
        mod_count: mods::get_mod_count(&conn).map_err(|e| e.to_string())?,
        character_count: characters::get_character_count(&conn).map_err(|e| e.to_string())?,
        quote_count: quotes::get_quote_count(&conn).map_err(|e| e.to_string())?,
    })
}
```

- [ ] **Step 2: Wire up database and command in lib.rs**

`src-tauri/src/lib.rs`:
```rust
mod db;
mod fetcher;
mod game;
mod commands;

use db::connection::Database;
use db::schema;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data = app.path().app_data_dir().expect("no app data dir");
            std::fs::create_dir_all(&app_data).expect("cannot create app data dir");
            let db_path = app_data.join("warframedle.db");
            let database = Database::new(&db_path).expect("cannot open database");
            {
                let conn = database.conn.lock().unwrap();
                schema::create_tables(&conn).expect("cannot create tables");
            }
            app.manage(database);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::get_db_stats])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Update React App to call the command**

`src/App.tsx`:
```tsx
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface DbStats {
  warframe_count: number;
  ability_count: number;
  weapon_count: number;
  mod_count: number;
  character_count: number;
  quote_count: number;
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
            <li>Characters: {stats.character_count}</li>
            <li>Quotes: {stats.quote_count}</li>
          </ul>
        </div>
      )}
      {!stats && !error && <p>Connecting to database...</p>}
    </div>
  );
}

export default App;
```

- [ ] **Step 4: Build and verify**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo build
```

Expected: compiles successfully. If you can run `npm run tauri dev`, the app window should show "Warframedle" with all counts at 0 (empty database).

- [ ] **Step 5: Run full test suite one final time**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src/App.tsx
git commit -m "feat: add Tauri IPC commands and wire up database stats to React"
```

---

## Phase 1 Complete

At this point you have:
- A Tauri v2 + React project that builds and runs
- SQLite database with all 32 tables created on startup
- Tier 1 query functions (insert, get_by_id, get_count, get_random) for warframes, abilities, weapons, mods, characters, and quotes — all tested
- A working IPC command that React can call to get database stats
- Clean git history with logical commits

**Next phase:** Phase 2 (Data Fetcher) will implement the MediaWiki API integration to populate these tables with real Warframe Wiki data.
