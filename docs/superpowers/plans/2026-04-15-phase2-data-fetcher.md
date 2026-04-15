# Phase 2: Data Fetcher — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the fetcher module that pulls real Warframe game data from the wiki's Lua data modules into the local SQLite database, with image downloads.

**Architecture:** Category-based pipeline. Each of 12 data categories has an independent fetcher module. A coordinator runs them in FK-safe order, emitting progress events. Infrastructure: HTTP client with rate limiting (`reqwest`), Lua interpreter (`mlua`) converts wiki Lua source to JSON, category modules map JSON fields to DB upserts.

**Tech Stack:** reqwest 0.12 (blocking, json), mlua 0.10 (lua54, vendored), serde_json

> **Phase scope:** Phase 2 of 5. Depends on Phase 1 (complete). Phase 3 (Game Engine) comes next.

---

## File Structure

```
src-tauri/src/fetcher/
├── mod.rs                — CategoryReport, CategoryResult, ImageTask types + module declarations
├── coordinator.rs        — runs categories in FK-safe order, emits Tauri progress events
├── wiki_client.rs        — HTTP: fetch_module_source(), resolve_image_urls(), download_image()
├── lua_parser.rs         — mlua → serde_json::Value conversion
├── image_downloader.rs   — batch URL resolution + sequential downloads
├── categories/
│   ├── mod.rs
│   ├── warframes.rs      — Module:Warframes/data → warframes table
│   ├── abilities.rs      — Module:Ability/data → abilities table (FK to warframes)
│   ├── weapons.rs        — Module:Weapons/data/* (8 sub-modules) → weapons table
│   ├── mods.rs           — Module:Mods/data → mods table
│   ├── companions.rs     — Module:Companions/data → companions + companion_precepts
│   ├── bosses.rs         — Module:Enemies/data/* (filter bosses) → bosses table
│   ├── planets.rs        — Module:Missions/data → planets + planet_resources
│   ├── factions.rs       — Module:Factions/data → syndicates table
│   ├── focus.rs          — Module:Focus/data → focus_schools + focus_abilities
│   ├── arcanes.rs        — Module:Arcane/data → arcanes table
│   ├── damage_types.rs   — Module:DamageTypes/data → elements + faction_weaknesses
│   └── relics.rs         — Module:Void/data → relics + relic_rewards
```

---

### Task 1: Schema Migration

**Files:**
- Modify: `src-tauri/src/db/schema.rs`
- Modify: `src-tauri/src/db/models.rs`
- Modify: `src-tauri/src/db/queries/mod.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/db/connection.rs`
- Delete: `src-tauri/src/db/queries/characters.rs`
- Delete: `src-tauri/src/db/queries/quotes.rs`

- [ ] **Step 1: Remove 10 CREATE TABLE statements from schema.rs**

Remove the following CREATE TABLE blocks from the `create_tables` function in `src-tauri/src/db/schema.rs`:
- `characters`
- `quotes`
- `quests`
- `progenitor_elements`
- `incarnon_weapons`
- `incarnon_evolutions`
- `railjack_intrinsics`
- `landing_craft`
- `lore_fragments`
- `cosmetics`

- [ ] **Step 2: Update schema test assertion**

In `schema.rs`, change `test_creates_all_tables`:
```rust
assert_eq!(count, 22);
```

Remove `test_abilities_foreign_key` test that references the characters table (if it does), or leave it if it only tests warframes→abilities FK.

- [ ] **Step 3: Delete character and quote query files**

Delete `src-tauri/src/db/queries/characters.rs` and `src-tauri/src/db/queries/quotes.rs`.

- [ ] **Step 4: Update queries/mod.rs**

Remove `pub mod characters;` and `pub mod quotes;` from `src-tauri/src/db/queries/mod.rs`.

- [ ] **Step 5: Remove Character and Quote from models.rs**

Delete the `Character` and `Quote` struct definitions from `src-tauri/src/db/models.rs`.

- [ ] **Step 6: Update commands/mod.rs**

Remove `characters` and `quotes` imports and fields from `DbStats`:

```rust
use crate::db::queries::{warframes, abilities, weapons, mods};

#[derive(Serialize)]
pub struct DbStats {
    pub warframe_count: i64,
    pub ability_count: i64,
    pub weapon_count: i64,
    pub mod_count: i64,
}

#[tauri::command]
pub fn get_db_stats(db: State<'_, Database>) -> Result<DbStats, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(DbStats {
        warframe_count: warframes::get_warframe_count(&conn).map_err(|e| e.to_string())?,
        ability_count: abilities::get_ability_count(&conn).map_err(|e| e.to_string())?,
        weapon_count: weapons::get_weapon_count(&conn).map_err(|e| e.to_string())?,
        mod_count: mods::get_mod_count(&conn).map_err(|e| e.to_string())?,
    })
}
```

- [ ] **Step 7: Update App.tsx to match new DbStats**

Remove `character_count` and `quote_count` from the `DbStats` interface and the JSX rendering in `src/App.tsx`.

- [ ] **Step 8: Add db_path field to Database struct**

Update `src-tauri/src/db/connection.rs` so the fetcher can open its own connection:

```rust
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct Database {
    pub conn: Mutex<Connection>,
    pub path: PathBuf,
}

impl Database {
    pub fn new(path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Ok(Database {
            conn: Mutex::new(conn),
            path: path.to_path_buf(),
        })
    }
}
```

- [ ] **Step 9: Run tests, verify they pass**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test
```

Expected: all remaining tests pass (schema tests expect 22 tables, warframe/ability/weapon/mod query tests still pass).

- [ ] **Step 10: Commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add -A
git commit -m "refactor: drop 10 tables without wiki data sources, remove character/quote code"
```

---

### Task 2: Add Dependencies + Wiki Client

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/fetcher/wiki_client.rs`
- Modify: `src-tauri/src/fetcher/mod.rs`

- [ ] **Step 1: Add reqwest and mlua to Cargo.toml**

Add to `[dependencies]` in `src-tauri/Cargo.toml`:
```toml
reqwest = { version = "0.12", features = ["blocking", "json"] }
mlua = { version = "0.10", features = ["lua54", "vendored"] }
```

- [ ] **Step 2: Create fetcher/mod.rs with types**

Replace the empty `src-tauri/src/fetcher/mod.rs` with:

```rust
pub mod wiki_client;

#[derive(Debug, Default)]
pub struct CategoryReport {
    pub category: String,
    pub inserted: usize,
    pub failed: usize,
}

#[derive(Debug)]
pub struct ImageTask {
    pub wiki_filename: String,
    pub local_subdir: String,
}
```

- [ ] **Step 3: Create wiki_client.rs**

`src-tauri/src/fetcher/wiki_client.rs`:

```rust
use reqwest::blocking::Client;
use serde_json::Value;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};
use std::cell::Cell;

pub struct WikiClient {
    client: Client,
    last_request: Cell<Instant>,
}

impl WikiClient {
    pub fn new() -> Self {
        WikiClient {
            client: Client::builder()
                .user_agent("Warframedle/0.1 (Desktop quiz game)")
                .build()
                .expect("failed to build HTTP client"),
            last_request: Cell::new(Instant::now() - Duration::from_secs(2)),
        }
    }

    fn rate_limit(&self) {
        let elapsed = self.last_request.get().elapsed();
        if elapsed < Duration::from_secs(1) {
            thread::sleep(Duration::from_secs(1) - elapsed);
        }
        self.last_request.set(Instant::now());
    }

    pub fn fetch_module_source(&self, module_name: &str) -> Result<String, String> {
        self.rate_limit();
        let resp: Value = self.client
            .get("https://warframe.fandom.com/api.php")
            .query(&[
                ("action", "query"),
                ("titles", module_name),
                ("prop", "revisions"),
                ("rvprop", "content"),
                ("rvslots", "main"),
                ("format", "json"),
            ])
            .send()
            .map_err(|e| format!("HTTP error: {e}"))?
            .json()
            .map_err(|e| format!("JSON parse error: {e}"))?;

        let pages = resp["query"]["pages"]
            .as_object()
            .ok_or("response missing query.pages")?;
        let page = pages.values().next().ok_or("no pages returned")?;

        if page.get("missing").is_some() {
            return Err(format!("Module not found: {module_name}"));
        }

        page["revisions"][0]["slots"]["main"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "no content in revision".to_string())
    }

    pub fn resolve_image_urls(&self, filenames: &[String]) -> Result<Vec<(String, String)>, String> {
        let mut results = Vec::new();
        for chunk in filenames.chunks(50) {
            self.rate_limit();
            let titles: String = chunk
                .iter()
                .map(|f| format!("File:{f}"))
                .collect::<Vec<_>>()
                .join("|");

            let resp: Value = self.client
                .get("https://warframe.fandom.com/api.php")
                .query(&[
                    ("action", "query"),
                    ("titles", &titles),
                    ("prop", "imageinfo"),
                    ("iiprop", "url"),
                    ("format", "json"),
                ])
                .send()
                .map_err(|e| format!("HTTP error: {e}"))?
                .json()
                .map_err(|e| format!("JSON parse error: {e}"))?;

            if let Some(pages) = resp["query"]["pages"].as_object() {
                for page in pages.values() {
                    if let (Some(title), Some(url)) = (
                        page["title"].as_str(),
                        page["imageinfo"][0]["url"].as_str(),
                    ) {
                        let filename = title.strip_prefix("File:").unwrap_or(title);
                        results.push((filename.to_string(), url.to_string()));
                    }
                }
            }
        }
        Ok(results)
    }

    pub fn download_image(&self, url: &str, local_path: &Path) -> Result<(), String> {
        if local_path.exists() {
            return Ok(());
        }
        self.rate_limit();
        let bytes = self.client
            .get(url)
            .send()
            .map_err(|e| format!("download error: {e}"))?
            .bytes()
            .map_err(|e| format!("read error: {e}"))?;

        if let Some(parent) = local_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir error: {e}"))?;
        }
        std::fs::write(local_path, &bytes).map_err(|e| format!("write error: {e}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires internet
    fn test_fetch_warframes_module() {
        let wiki = WikiClient::new();
        let source = wiki.fetch_module_source("Module:Warframes/data").unwrap();
        assert!(source.contains("Excalibur"));
        assert!(source.len() > 100_000);
    }

    #[test]
    #[ignore] // Requires internet
    fn test_resolve_image_url() {
        let wiki = WikiClient::new();
        let results = wiki.resolve_image_urls(&["Excalibur.png".to_string()]).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].1.contains("static.wikia.nocookie.net"));
    }
}
```

- [ ] **Step 4: Verify build**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo build
```

Expected: compiles (first build with reqwest+mlua will take a few minutes).

- [ ] **Step 5: Commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/fetcher/
git commit -m "feat: add wiki client with rate limiting for MediaWiki API"
```

---

### Task 3: Lua Parser

**Files:**
- Create: `src-tauri/src/fetcher/lua_parser.rs`
- Modify: `src-tauri/src/fetcher/mod.rs`

- [ ] **Step 1: Add lua_parser module declaration**

Add `pub mod lua_parser;` to `src-tauri/src/fetcher/mod.rs`.

- [ ] **Step 2: Create lua_parser.rs**

`src-tauri/src/fetcher/lua_parser.rs`:

```rust
use mlua::prelude::*;
use serde_json::{Map, Value};

pub fn eval_lua_module(source: &str) -> Result<Value, String> {
    let lua = Lua::new();

    // Provide a dummy require() so modules that reference other modules don't crash
    let dummy_require = lua
        .create_function(|lua_ctx, _name: String| lua_ctx.create_table())
        .map_err(|e| format!("failed to create dummy require: {e}"))?;
    lua.globals()
        .set("require", dummy_require)
        .map_err(|e| format!("failed to set require: {e}"))?;

    let table: LuaTable = lua
        .load(source)
        .eval()
        .map_err(|e| format!("Lua eval error: {e}"))?;

    lua_table_to_json(&table)
}

fn lua_table_to_json(table: &LuaTable) -> Result<Value, String> {
    let len = table.raw_len();

    // Check if array-like (sequential integer keys starting at 1)
    if len > 0 {
        let mut arr = Vec::with_capacity(len as usize);
        let mut is_array = true;
        for i in 1..=len {
            match table.raw_get::<LuaValue>(i) {
                Ok(val) if val != LuaValue::Nil => {
                    arr.push(lua_value_to_json(val)?);
                }
                _ => {
                    is_array = false;
                    break;
                }
            }
        }
        if is_array && arr.len() == len as usize {
            return Ok(Value::Array(arr));
        }
    }

    // Object
    let mut map = Map::new();
    for pair in table.pairs::<LuaValue, LuaValue>() {
        let (key, value) = pair.map_err(|e| format!("table iteration error: {e}"))?;
        let key_str = match &key {
            LuaValue::String(s) => s.to_str().map_err(|e| format!("key encode error: {e}"))?.to_string(),
            LuaValue::Integer(i) => i.to_string(),
            _ => continue,
        };
        map.insert(key_str, lua_value_to_json(value)?);
    }
    Ok(Value::Object(map))
}

fn lua_value_to_json(value: LuaValue) -> Result<Value, String> {
    match value {
        LuaValue::Nil => Ok(Value::Null),
        LuaValue::Boolean(b) => Ok(Value::Bool(b)),
        LuaValue::Integer(i) => Ok(Value::Number(i.into())),
        LuaValue::Number(f) => Ok(serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null)),
        LuaValue::String(s) => Ok(Value::String(
            s.to_str().map_err(|e| format!("string encode error: {e}"))?.to_string(),
        )),
        LuaValue::Table(t) => lua_table_to_json(&t),
        _ => Ok(Value::Null), // Functions, userdata, etc. → null
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_table() {
        let data = eval_lua_module(r#"return { Name = "Excalibur", Health = 100 }"#).unwrap();
        assert_eq!(data["Name"], "Excalibur");
        assert_eq!(data["Health"], 100);
    }

    #[test]
    fn test_nested_table() {
        let data = eval_lua_module(
            r#"return { ["Excalibur"] = { Name = "Excalibur", Type = "Warframe", Health = 100, Sprint = 1.0 } }"#,
        ).unwrap();
        assert_eq!(data["Excalibur"]["Name"], "Excalibur");
        assert_eq!(data["Excalibur"]["Health"], 100);
        assert_eq!(data["Excalibur"]["Sprint"], 1.0);
    }

    #[test]
    fn test_array_field() {
        let data = eval_lua_module(
            r#"return { Abilities = {"Slash Dash", "Radial Blind", "Radial Javelin", "Exalted Blade"} }"#,
        ).unwrap();
        let abilities = data["Abilities"].as_array().unwrap();
        assert_eq!(abilities.len(), 4);
        assert_eq!(abilities[0], "Slash Dash");
    }

    #[test]
    fn test_nil_and_bool() {
        let data = eval_lua_module(r#"return { Vaulted = true, Missing = nil }"#).unwrap();
        assert_eq!(data["Vaulted"], true);
        assert!(data["Missing"].is_null());
    }

    #[test]
    fn test_empty_table() {
        let data = eval_lua_module(r#"return {}"#).unwrap();
        assert!(data.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_module_with_require() {
        // Modules that use require() should get an empty table back, not crash
        let data = eval_lua_module(
            r#"local utils = require("Module:Utils"); return { Name = "Test" }"#,
        ).unwrap();
        assert_eq!(data["Name"], "Test");
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test fetcher::lua_parser -- --nocapture
```

Expected: all 6 tests pass.

- [ ] **Step 4: Commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add src-tauri/src/fetcher/
git commit -m "feat: add Lua parser that converts wiki Lua modules to JSON"
```

---

### Task 4: Category Infrastructure + Warframes Fetcher

**Files:**
- Modify: `src-tauri/src/fetcher/mod.rs`
- Create: `src-tauri/src/fetcher/categories/mod.rs`
- Create: `src-tauri/src/fetcher/categories/warframes.rs`

- [ ] **Step 1: Update fetcher/mod.rs with categories module**

```rust
pub mod wiki_client;
pub mod lua_parser;
pub mod categories;

#[derive(Debug, Default)]
pub struct CategoryReport {
    pub category: String,
    pub inserted: usize,
    pub failed: usize,
}

#[derive(Debug)]
pub struct ImageTask {
    pub wiki_filename: String,
    pub local_subdir: String,
}

pub struct CategoryResult {
    pub report: CategoryReport,
    pub images: Vec<ImageTask>,
}
```

- [ ] **Step 2: Create categories/mod.rs**

```rust
pub mod warframes;
```

- [ ] **Step 3: Create warframes.rs**

`src-tauri/src/fetcher/categories/warframes.rs`:

```rust
use rusqlite::{params, Connection};
use serde_json::Value;
use crate::fetcher::{CategoryResult, CategoryReport, ImageTask};
use crate::fetcher::wiki_client::WikiClient;
use crate::fetcher::lua_parser;

pub fn fetch_warframes(
    conn: &Connection,
    wiki: &WikiClient,
) -> Result<CategoryResult, String> {
    let source = wiki.fetch_module_source("Module:Warframes/data")?;
    let data = lua_parser::eval_lua_module(&source)?;
    process_warframes_data(conn, &data)
}

pub fn process_warframes_data(
    conn: &Connection,
    data: &Value,
) -> Result<CategoryResult, String> {
    let entries = data.as_object().ok_or("warframes data: expected object")?;
    let mut report = CategoryReport { category: "warframes".to_string(), ..Default::default() };
    let mut images = Vec::new();

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    for (_key, entry) in entries {
        let name = match entry["Name"].as_str() {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };
        let wf_type = entry["Type"].as_str().unwrap_or("Warframe");
        let description = entry["Description"].as_str().unwrap_or_default();
        let health = entry["Health"].as_f64();
        let shields = entry["Shield"].as_f64();
        let armor = entry["Armor"].as_f64();
        let energy = entry["Energy"].as_f64();
        let sprint_speed = entry["Sprint"].as_f64();
        let passive = entry["Passive"].as_str().unwrap_or_default();
        let mastery_rank = entry["Mastery"].as_i64().map(|v| v as i32);
        let acquisition = entry["Acquisition"].as_str().unwrap_or_default();
        let release_date = entry["Introduced"].as_str().map(|s| s.to_string());
        let is_vaulted = entry["Vaulted"].as_bool().unwrap_or(false);
        let helminth_ability = entry["Subsumed"].as_str().map(|s| s.to_string());
        let sex = entry["Sex"].as_str().map(|s| s.to_string());
        let image = entry["Image"].as_str().map(|s| s.to_string());

        let icon_path = image.as_ref().map(|img| format!("assets/warframes/{img}"));

        match tx.execute(
            "INSERT INTO warframes (name, type, description, health, shields, armor, energy,
             sprint_speed, passive, mastery_rank, acquisition, release_date,
             is_vaulted, helminth_ability, sex, icon_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
             ON CONFLICT(name) DO UPDATE SET
             type=excluded.type, description=excluded.description, health=excluded.health,
             shields=excluded.shields, armor=excluded.armor, energy=excluded.energy,
             sprint_speed=excluded.sprint_speed, passive=excluded.passive,
             mastery_rank=excluded.mastery_rank, acquisition=excluded.acquisition,
             release_date=excluded.release_date, is_vaulted=excluded.is_vaulted,
             helminth_ability=excluded.helminth_ability, sex=excluded.sex,
             icon_path=excluded.icon_path",
            params![name, wf_type, description, health, shields, armor, energy,
                    sprint_speed, passive, mastery_rank, acquisition, release_date,
                    is_vaulted, helminth_ability, sex, icon_path],
        ) {
            Ok(_) => report.inserted += 1,
            Err(e) => {
                eprintln!("Failed to upsert warframe {name}: {e}");
                report.failed += 1;
            }
        }

        if let Some(img) = image {
            images.push(ImageTask { wiki_filename: img, local_subdir: "warframes".to_string() });
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(CategoryResult { report, images })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;
    use crate::fetcher::lua_parser;

    #[test]
    fn test_process_warframes() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Excalibur"] = {
                Name = "Excalibur", Type = "Warframe", Health = 100,
                Shield = 100, Armor = 225, Energy = 100, Sprint = 1.0,
                Passive = "Swordsmanship", Description = "A balanced fighter",
                Image = "Excalibur.png", Sex = "Male", Mastery = 0,
                Vaulted = false
            },
            ["Amesha"] = {
                Name = "Amesha", Type = "Archwing", Health = 100,
                Shield = 100, Armor = 50, Energy = 100, Sprint = 1.0,
                Passive = "", Description = "Support archwing",
                Image = "Amesha.png"
            }
        }"#).unwrap();

        let result = process_warframes_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 2);
        assert_eq!(result.report.failed, 0);
        assert_eq!(result.images.len(), 2);

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM warframes", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 2);

        let name: String = conn.query_row(
            "SELECT name FROM warframes WHERE type = 'Archwing'", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(name, "Amesha");
    }

    #[test]
    fn test_upsert_updates_existing() {
        let conn = test_db();
        let data1 = lua_parser::eval_lua_module(r#"return {
            ["Excalibur"] = { Name = "Excalibur", Type = "Warframe", Health = 100 }
        }"#).unwrap();
        process_warframes_data(&conn, &data1).unwrap();

        let data2 = lua_parser::eval_lua_module(r#"return {
            ["Excalibur"] = { Name = "Excalibur", Type = "Warframe", Health = 370 }
        }"#).unwrap();
        process_warframes_data(&conn, &data2).unwrap();

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM warframes", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
        let health: f64 = conn.query_row("SELECT health FROM warframes WHERE name = 'Excalibur'", [], |r| r.get(0)).unwrap();
        assert_eq!(health, 370.0);
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test fetcher::categories::warframes -- --nocapture
```

Expected: both tests pass.

- [ ] **Step 5: Commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add src-tauri/src/fetcher/
git commit -m "feat: add warframes category fetcher with Lua-to-DB mapping"
```

---

### Task 5: Abilities + Weapons Fetchers

**Files:**
- Create: `src-tauri/src/fetcher/categories/abilities.rs`
- Create: `src-tauri/src/fetcher/categories/weapons.rs`
- Modify: `src-tauri/src/fetcher/categories/mod.rs`

- [ ] **Step 1: Update categories/mod.rs**

```rust
pub mod warframes;
pub mod abilities;
pub mod weapons;
```

- [ ] **Step 2: Create abilities.rs**

`src-tauri/src/fetcher/categories/abilities.rs`:

```rust
use rusqlite::{params, Connection};
use serde_json::Value;
use crate::fetcher::{CategoryResult, CategoryReport, ImageTask};
use crate::fetcher::wiki_client::WikiClient;
use crate::fetcher::lua_parser;

pub fn fetch_abilities(conn: &Connection, wiki: &WikiClient) -> Result<CategoryResult, String> {
    let source = wiki.fetch_module_source("Module:Ability/data")?;
    let data = lua_parser::eval_lua_module(&source)?;
    process_abilities_data(conn, &data)
}

pub fn process_abilities_data(conn: &Connection, data: &Value) -> Result<CategoryResult, String> {
    let mut report = CategoryReport { category: "abilities".to_string(), ..Default::default() };
    let mut images = Vec::new();
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    // The module may have a top-level "Warframe" subtable, or abilities directly at root
    let entries = if let Some(wf_section) = data.get("Warframe").and_then(|v| v.as_object()) {
        wf_section.clone()
    } else {
        data.as_object().ok_or("abilities data: expected object")?.clone()
    };

    for (_key, entry) in &entries {
        let name = match entry["Name"].as_str() {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };
        let cost = entry["Cost"].as_i64().map(|v| v as i32);
        let description = entry["Description"].as_str().unwrap_or_default();
        let icon = entry["Icon"].as_str().map(|s| s.to_string());
        let powersuit = entry["PowerSuit"].as_str().unwrap_or_default();
        let slot_index = entry["Key"].as_i64().map(|v| v as i32);
        let is_helminth = entry["Subsumable"].as_bool().unwrap_or(false);
        let augment = entry["Augment"].as_str().map(|s| s.to_string());

        // FK lookup: find warframe_id by name
        let warframe_id: Option<i64> = if !powersuit.is_empty() {
            tx.query_row(
                "SELECT id FROM warframes WHERE name = ?1",
                params![powersuit],
                |row| row.get(0),
            ).ok()
        } else {
            None
        };

        let warframe_id = match warframe_id {
            Some(id) => id,
            None => {
                report.failed += 1;
                continue;
            }
        };

        let icon_path = icon.as_ref().map(|i| format!("assets/abilities/{i}"));

        match tx.execute(
            "INSERT INTO abilities (name, cost, description, icon_path, warframe_id, slot_index, is_helminth, augment_mod_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(name, warframe_id) DO UPDATE SET
             cost=excluded.cost, description=excluded.description, icon_path=excluded.icon_path,
             slot_index=excluded.slot_index, is_helminth=excluded.is_helminth,
             augment_mod_name=excluded.augment_mod_name",
            params![name, cost, description, icon_path, warframe_id, slot_index, is_helminth, augment],
        ) {
            Ok(_) => report.inserted += 1,
            Err(e) => {
                eprintln!("Failed to upsert ability {name}: {e}");
                report.failed += 1;
            }
        }

        if let Some(img) = icon {
            images.push(ImageTask { wiki_filename: img, local_subdir: "abilities".to_string() });
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(CategoryResult { report, images })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;
    use crate::fetcher::lua_parser;

    fn insert_test_warframe(conn: &Connection) {
        conn.execute(
            "INSERT INTO warframes (name, type) VALUES ('Excalibur', 'Warframe')", [],
        ).unwrap();
    }

    #[test]
    fn test_process_abilities() {
        let conn = test_db();
        insert_test_warframe(&conn);

        let data = lua_parser::eval_lua_module(r#"return {
            ["Slash Dash"] = {
                Name = "Slash Dash", PowerSuit = "Excalibur", Cost = 25,
                Description = "Dash forward", Key = 1, Icon = "SlashDash.png"
            },
            ["Radial Blind"] = {
                Name = "Radial Blind", PowerSuit = "Excalibur", Cost = 50,
                Description = "Blind enemies", Key = 2
            }
        }"#).unwrap();

        let result = process_abilities_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 2);

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM abilities", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_ability_missing_warframe_skipped() {
        let conn = test_db();
        // No warframe inserted — ability should fail FK lookup
        let data = lua_parser::eval_lua_module(r#"return {
            ["Test"] = { Name = "Test", PowerSuit = "NonExistent", Cost = 25 }
        }"#).unwrap();

        let result = process_abilities_data(&conn, &data).unwrap();
        assert_eq!(result.report.inserted, 0);
        assert_eq!(result.report.failed, 1);
    }
}
```

**Note:** The abilities table needs a UNIQUE constraint on `(name, warframe_id)` for the ON CONFLICT to work. If the current schema only has `name` without UNIQUE, the subagent should add a unique index or use DELETE+INSERT instead. Check `schema.rs` and adjust the upsert SQL accordingly — the simplest approach is to delete all abilities for a warframe before re-inserting.

- [ ] **Step 3: Run ability tests**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test fetcher::categories::abilities -- --nocapture
```

Expected: both tests pass.

- [ ] **Step 4: Create weapons.rs**

`src-tauri/src/fetcher/categories/weapons.rs`:

```rust
use rusqlite::{params, Connection};
use serde_json::Value;
use crate::fetcher::{CategoryResult, CategoryReport, ImageTask};
use crate::fetcher::wiki_client::WikiClient;
use crate::fetcher::lua_parser;

const WEAPON_MODULES: &[(&str, &str)] = &[
    ("Module:Weapons/data/primary", "Primary"),
    ("Module:Weapons/data/secondary", "Secondary"),
    ("Module:Weapons/data/melee", "Melee"),
    ("Module:Weapons/data/archwing", "Archwing"),
    ("Module:Weapons/data/companion", "Companion"),
    ("Module:Weapons/data/railjack", "Railjack"),
    ("Module:Weapons/data/modular", "Modular"),
    ("Module:Weapons/data/misc", "Misc"),
];

pub fn fetch_weapons(conn: &Connection, wiki: &WikiClient) -> Result<CategoryResult, String> {
    let mut all_report = CategoryReport { category: "weapons".to_string(), ..Default::default() };
    let mut all_images = Vec::new();

    for (module, weapon_type) in WEAPON_MODULES {
        match wiki.fetch_module_source(module) {
            Ok(source) => {
                let data = lua_parser::eval_lua_module(&source)?;
                let result = process_weapons_data(conn, &data, weapon_type)?;
                all_report.inserted += result.report.inserted;
                all_report.failed += result.report.failed;
                all_images.extend(result.images);
            }
            Err(e) => {
                eprintln!("Failed to fetch {module}: {e}");
            }
        }
    }

    Ok(CategoryResult { report: all_report, images: all_images })
}

pub fn process_weapons_data(
    conn: &Connection,
    data: &Value,
    default_type: &str,
) -> Result<CategoryResult, String> {
    let entries = data.as_object().ok_or("weapons data: expected object")?;
    let mut report = CategoryReport { category: "weapons".to_string(), ..Default::default() };
    let mut images = Vec::new();
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    for (_key, entry) in entries {
        let name = match entry["Name"].as_str() {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };
        let weapon_type = entry["Type"].as_str().unwrap_or(default_type);
        let subtype = entry["Class"].as_str().unwrap_or_default();
        let mastery = entry["Mastery"].as_i64().map(|v| v as i32);
        let crit_chance = entry["CritChance"].as_f64();
        let crit_mult = entry["CritMultiplier"].as_f64();
        let status_chance = entry["StatusChance"].as_f64();
        let fire_rate = entry["FireRate"].as_f64();
        let magazine = entry["Magazine"].as_i64().map(|v| v as i32);
        let reload = entry["Reload"].as_f64();
        let trigger = entry["Trigger"].as_str().map(|s| s.to_string());
        let disposition = entry["Disposition"].as_f64();
        let image = entry["Image"].as_str().map(|s| s.to_string());

        // Parse damage from nested table
        let damage_total = entry["Damage"].as_object()
            .map(|d| d.values().filter_map(|v| v.as_f64()).sum::<f64>());
        let damage_impact = entry["Damage"].as_object()
            .and_then(|d| d.get("Impact")).and_then(|v| v.as_f64());
        let damage_puncture = entry["Damage"].as_object()
            .and_then(|d| d.get("Puncture")).and_then(|v| v.as_f64());
        let damage_slash = entry["Damage"].as_object()
            .and_then(|d| d.get("Slash")).and_then(|v| v.as_f64());

        let icon_path = image.as_ref().map(|img| format!("assets/weapons/{img}"));

        match tx.execute(
            "INSERT INTO weapons (name, type, subtype, mastery_rank, damage_total,
             damage_impact, damage_puncture, damage_slash, crit_chance, crit_multiplier,
             status_chance, fire_rate, magazine_size, reload_time, trigger_type,
             riven_disposition, icon_path)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)
             ON CONFLICT(name) DO UPDATE SET
             type=excluded.type, subtype=excluded.subtype, mastery_rank=excluded.mastery_rank,
             damage_total=excluded.damage_total, damage_impact=excluded.damage_impact,
             damage_puncture=excluded.damage_puncture, damage_slash=excluded.damage_slash,
             crit_chance=excluded.crit_chance, crit_multiplier=excluded.crit_multiplier,
             status_chance=excluded.status_chance, fire_rate=excluded.fire_rate,
             magazine_size=excluded.magazine_size, reload_time=excluded.reload_time,
             trigger_type=excluded.trigger_type, riven_disposition=excluded.riven_disposition,
             icon_path=excluded.icon_path",
            params![name, weapon_type, subtype, mastery, damage_total,
                    damage_impact, damage_puncture, damage_slash, crit_chance, crit_mult,
                    status_chance, fire_rate, magazine, reload, trigger,
                    disposition, icon_path],
        ) {
            Ok(_) => report.inserted += 1,
            Err(e) => {
                eprintln!("Failed to upsert weapon {name}: {e}");
                report.failed += 1;
            }
        }

        if let Some(img) = image {
            images.push(ImageTask { wiki_filename: img, local_subdir: "weapons".to_string() });
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(CategoryResult { report, images })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::test_db;
    use crate::fetcher::lua_parser;

    #[test]
    fn test_process_weapons() {
        let conn = test_db();
        let data = lua_parser::eval_lua_module(r#"return {
            ["Boltor"] = {
                Name = "Boltor", Type = "Primary", Class = "Rifle",
                Mastery = 2, CritChance = 0.1, CritMultiplier = 1.6,
                StatusChance = 0.14, FireRate = 8.75, Magazine = 60, Reload = 2.6,
                Trigger = "Auto", Disposition = 1.3, Image = "Boltor.png",
                Damage = { Impact = 2.5, Puncture = 20, Slash = 2.5 }
            }
        }"#).unwrap();

        let result = process_weapons_data(&conn, &data, "Primary").unwrap();
        assert_eq!(result.report.inserted, 1);

        let total: f64 = conn.query_row(
            "SELECT damage_total FROM weapons WHERE name = 'Boltor'", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(total, 25.0);
    }
}
```

- [ ] **Step 5: Run weapon tests**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test fetcher::categories::weapons -- --nocapture
```

- [ ] **Step 6: Run all tests**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test
```

- [ ] **Step 7: Commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add src-tauri/src/fetcher/
git commit -m "feat: add abilities and weapons category fetchers"
```

---

### Task 6: Mods + Companions + Bosses Fetchers

**Files:**
- Create: `src-tauri/src/fetcher/categories/mods.rs`
- Create: `src-tauri/src/fetcher/categories/companions.rs`
- Create: `src-tauri/src/fetcher/categories/bosses.rs`
- Modify: `src-tauri/src/fetcher/categories/mod.rs`

- [ ] **Step 1: Add module declarations to categories/mod.rs**

Add `pub mod mods;`, `pub mod companions;`, `pub mod bosses;`

- [ ] **Step 2: Create mods.rs**

Follow the same pattern as warframes.rs. Key differences:
- Module: `Module:Mods/data`
- Lua fields: Name, Polarity, Rarity, Type→mod_type, MaxRank, BaseDrain, Description→effect_description, IsExilus, IsAbilityAugment→is_augment, Image
- Upsert ON CONFLICT(name)
- For augment mods, look up `augment_warframe_id` from the `PowerSuit` or `Augment` field if present

Test with sample Lua containing a mod entry like:
```lua
["Serration"] = { Name = "Serration", Polarity = "Madurai", Rarity = "Rare", Type = "Rifle", MaxRank = 10, BaseDrain = 4, Description = "+Damage", Image = "Serration.png" }
```

- [ ] **Step 3: Create companions.rs**

- Module: `Module:Companions/data`
- Lua fields: Name, Type→class, Health, Armor, Shield→shields, Description, Image, Mastery→mastery_rank
- Upsert ON CONFLICT(name) into companions table
- If the Lua data contains precept/ability info, also insert into companion_precepts (delete existing precepts for the companion first, then re-insert)

Test with sample Lua.

- [ ] **Step 4: Create bosses.rs**

- Modules: `Module:Enemies/data/corpus`, `Module:Enemies/data/grineer`, etc. (11 sub-modules, same pattern as weapons with multiple sub-modules)
- Filter: only keep entries where the Type or IsAssassin field indicates a boss
- Lua fields: Name, Faction (from sub-module name), Description, Image
- Upsert ON CONFLICT(name) into bosses table

Test with sample Lua containing a boss entry.

- [ ] **Step 5: Run all fetcher tests**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test fetcher::categories -- --nocapture
```

- [ ] **Step 6: Commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add src-tauri/src/fetcher/
git commit -m "feat: add mods, companions, bosses category fetchers"
```

---

### Task 7: Planets + Factions + Focus Fetchers

**Files:**
- Create: `src-tauri/src/fetcher/categories/planets.rs`
- Create: `src-tauri/src/fetcher/categories/factions.rs`
- Create: `src-tauri/src/fetcher/categories/focus.rs`
- Modify: `src-tauri/src/fetcher/categories/mod.rs`

- [ ] **Step 1: Add module declarations**

Add `pub mod planets;`, `pub mod factions;`, `pub mod focus;`

- [ ] **Step 2: Create planets.rs**

- Module: `Module:Missions/data`
- Complex nested structure: the module contains planet data with mission nodes
- Extract planets (name, faction, tileset) into `planets` table
- Extract resources per planet into `planet_resources` table (delete existing resources for each planet, re-insert)
- Upsert planets ON CONFLICT(name)

Test with sample Lua representing one planet with a few nodes and resources.

- [ ] **Step 3: Create factions.rs**

- Module: `Module:Factions/data`
- Lua fields: Name, Description, Image
- Maps to `syndicates` table (factions and syndicates share this table)
- Upsert ON CONFLICT(name)
- Leave `syndicate_relations` empty (data not available in Lua module)

Test with sample Lua.

- [ ] **Step 4: Create focus.rs**

- Module: `Module:Focus/data`
- Two-level insertion: first upsert focus schools, then their abilities
- Schools: Name, Description, Image → `focus_schools` table
- Abilities: Name, Description, IsWayBound, IsPassive, school_id FK → `focus_abilities` table
- Delete existing focus_abilities for each school before re-inserting

Test with sample Lua containing one school with two abilities.

- [ ] **Step 5: Run tests and commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test fetcher::categories -- --nocapture
```

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add src-tauri/src/fetcher/
git commit -m "feat: add planets, factions, focus category fetchers"
```

---

### Task 8: Arcanes + Damage Types + Relics Fetchers

**Files:**
- Create: `src-tauri/src/fetcher/categories/arcanes.rs`
- Create: `src-tauri/src/fetcher/categories/damage_types.rs`
- Create: `src-tauri/src/fetcher/categories/relics.rs`
- Modify: `src-tauri/src/fetcher/categories/mod.rs`

- [ ] **Step 1: Add module declarations**

Add `pub mod arcanes;`, `pub mod damage_types;`, `pub mod relics;`

- [ ] **Step 2: Create arcanes.rs**

- Module: `Module:Arcane/data`
- Lua fields: Name, Description→effect, Criteria→trigger_condition, MaxRank→max_rank, Rarity→source, Type→equipment_type, Image
- Upsert ON CONFLICT(name) into `arcanes` table

Test with sample Lua.

- [ ] **Step 3: Create damage_types.rs**

- Module: `Module:DamageTypes/data`
- Maps to TWO tables: `elements` and `faction_weaknesses`
- Elements: Name, element_type (physical/primary/combined), status_effect, component_a, component_b
- Faction weaknesses: extract from Positives/Negatives data in the damage type entries
- Delete all existing elements and faction_weaknesses, then re-insert (small static dataset, full replacement is fine)

Test with sample Lua containing one base element and one combined element.

- [ ] **Step 4: Create relics.rs**

- Module: `Module:Void/data`
- Two-level insertion: upsert relics, then replace their rewards
- Relics: Name, Tier→era, Vaulted→is_vaulted → `relics` table
- Rewards: Drops array → `relic_rewards` table (delete existing rewards per relic, re-insert)
- Each drop has: Item, Part (combine into item_name), Rarity

Test with sample Lua containing one relic with 6 rewards.

- [ ] **Step 5: Run tests and commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test fetcher::categories -- --nocapture
```

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add src-tauri/src/fetcher/
git commit -m "feat: add arcanes, damage types, relics category fetchers"
```

---

### Task 9: Image Downloader + Coordinator

**Files:**
- Create: `src-tauri/src/fetcher/image_downloader.rs`
- Create: `src-tauri/src/fetcher/coordinator.rs`
- Modify: `src-tauri/src/fetcher/mod.rs`

- [ ] **Step 1: Add module declarations to fetcher/mod.rs**

Add `pub mod image_downloader;` and `pub mod coordinator;`

- [ ] **Step 2: Create image_downloader.rs**

`src-tauri/src/fetcher/image_downloader.rs`:

```rust
use std::path::Path;
use crate::fetcher::wiki_client::WikiClient;
use crate::fetcher::ImageTask;

pub fn download_images(
    wiki: &WikiClient,
    tasks: &[ImageTask],
    assets_dir: &Path,
) -> (usize, usize) {
    if tasks.is_empty() {
        return (0, 0);
    }

    let filenames: Vec<String> = tasks.iter().map(|t| t.wiki_filename.clone()).collect();
    let url_map: std::collections::HashMap<String, String> = match wiki.resolve_image_urls(&filenames) {
        Ok(pairs) => pairs.into_iter().collect(),
        Err(e) => {
            eprintln!("Failed to resolve image URLs: {e}");
            return (0, tasks.len());
        }
    };

    let mut downloaded = 0;
    let mut failed = 0;

    for task in tasks {
        if let Some(url) = url_map.get(&task.wiki_filename) {
            let local_path = assets_dir.join(&task.local_subdir).join(&task.wiki_filename);
            match wiki.download_image(url, &local_path) {
                Ok(()) => downloaded += 1,
                Err(e) => {
                    eprintln!("Failed to download {}: {e}", task.wiki_filename);
                    failed += 1;
                }
            }
        } else {
            failed += 1;
        }
    }

    (downloaded, failed)
}
```

- [ ] **Step 3: Create coordinator.rs**

`src-tauri/src/fetcher/coordinator.rs`:

```rust
use rusqlite::Connection;
use serde::Serialize;
use std::path::Path;
use crate::fetcher::wiki_client::WikiClient;
use crate::fetcher::image_downloader;
use crate::fetcher::categories;
use crate::fetcher::{CategoryReport, CategoryResult, ImageTask};

#[derive(Debug, Serialize)]
pub struct FetchReport {
    pub categories: Vec<CategoryReport>,
    pub images_downloaded: usize,
    pub images_failed: usize,
}

#[derive(Serialize, Clone)]
pub struct FetchProgress {
    pub category: String,
    pub status: String,
    pub current: usize,
    pub total: usize,
    pub message: String,
}

type FetchFn = fn(&Connection, &WikiClient) -> Result<CategoryResult, String>;

const CATEGORIES: &[(&str, FetchFn)] = &[
    ("warframes", categories::warframes::fetch_warframes),
    ("abilities", categories::abilities::fetch_abilities),
    ("weapons", categories::weapons::fetch_weapons),
    ("mods", categories::mods::fetch_mods),
    ("companions", categories::companions::fetch_companions),
    ("bosses", categories::bosses::fetch_bosses),
    ("planets", categories::planets::fetch_planets),
    ("factions", categories::factions::fetch_factions),
    ("focus", categories::focus::fetch_focus),
    ("arcanes", categories::arcanes::fetch_arcanes),
    ("damage_types", categories::damage_types::fetch_damage_types),
    ("relics", categories::relics::fetch_relics),
];

pub fn fetch_all(
    conn: &Connection,
    assets_dir: &Path,
    emit_progress: &dyn Fn(FetchProgress),
) -> FetchReport {
    let wiki = WikiClient::new();
    let total = CATEGORIES.len();
    let mut reports = Vec::new();
    let mut all_images: Vec<ImageTask> = Vec::new();

    for (i, (name, fetch_fn)) in CATEGORIES.iter().enumerate() {
        emit_progress(FetchProgress {
            category: name.to_string(),
            status: "fetching".to_string(),
            current: i + 1,
            total,
            message: format!("Fetching {name}..."),
        });

        match fetch_fn(conn, &wiki) {
            Ok(result) => {
                all_images.extend(result.images);
                emit_progress(FetchProgress {
                    category: name.to_string(),
                    status: "done".to_string(),
                    current: i + 1,
                    total,
                    message: format!("{}: {} records", name, result.report.inserted),
                });
                reports.push(result.report);
            }
            Err(e) => {
                eprintln!("Category {name} failed: {e}");
                emit_progress(FetchProgress {
                    category: name.to_string(),
                    status: "error".to_string(),
                    current: i + 1,
                    total,
                    message: format!("{name} failed: {e}"),
                });
                reports.push(CategoryReport {
                    category: name.to_string(),
                    failed: 1,
                    ..Default::default()
                });
            }
        }
    }

    // Download images
    emit_progress(FetchProgress {
        category: "images".to_string(),
        status: "downloading_images".to_string(),
        current: total,
        total,
        message: format!("Downloading {} images...", all_images.len()),
    });

    let (downloaded, img_failed) = image_downloader::download_images(&wiki, &all_images, assets_dir);

    FetchReport {
        categories: reports,
        images_downloaded: downloaded,
        images_failed: img_failed,
    }
}
```

- [ ] **Step 4: Verify build**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo build
```

- [ ] **Step 5: Commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add src-tauri/src/fetcher/
git commit -m "feat: add image downloader and fetch coordinator"
```

---

### Task 10: Tauri Command Integration

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/App.tsx`

- [ ] **Step 1: Add fetch_wiki_data command**

Add to `src-tauri/src/commands/mod.rs`:

```rust
use serde::Serialize;
use tauri::{State, AppHandle, Emitter};
use crate::db::connection::Database;
use crate::db::queries::{warframes, abilities, weapons, mods};
use crate::db::schema;
use crate::fetcher::coordinator::{self, FetchReport, FetchProgress};
use rusqlite::Connection;
use std::path::Path;

// ... keep existing DbStats and get_db_stats ...

#[tauri::command]
pub fn fetch_wiki_data(db: State<'_, Database>, app: AppHandle) -> Result<FetchReport, String> {
    // Open a fresh DB connection for the fetch (avoids locking the main connection)
    let conn = Connection::open(&db.path).map_err(|e| format!("DB open error: {e}"))?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|e| format!("PRAGMA error: {e}"))?;
    schema::create_tables(&conn).map_err(|e| format!("schema error: {e}"))?;

    let assets_dir = db.path.parent().unwrap_or(Path::new(".")).join("assets");

    let report = coordinator::fetch_all(&conn, &assets_dir, &|progress: FetchProgress| {
        let _ = app.emit("fetch_progress", &progress);
    });

    Ok(report)
}
```

- [ ] **Step 2: Register command in lib.rs**

Update the `invoke_handler` in `src-tauri/src/lib.rs`:

```rust
.invoke_handler(tauri::generate_handler![
    commands::get_db_stats,
    commands::fetch_wiki_data
])
```

- [ ] **Step 3: Update App.tsx with fetch button**

Add a "Fetch Data" button to `src/App.tsx` that calls the fetch command and listens for progress events:

```tsx
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
```

- [ ] **Step 4: Verify build**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo build
```

- [ ] **Step 5: Run full test suite**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle/src-tauri
cargo test
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
cd /c/Users/erwan/Desktop/FOLDERS/python/warframedle
git add src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src/App.tsx
git commit -m "feat: wire up fetch_wiki_data Tauri command with progress events"
```

---

## Phase 2 Complete

At this point you have:
- Schema migrated to 22 tables (10 dropped)
- Wiki client with rate limiting
- Lua parser converting wiki Lua modules to JSON
- 12 category fetcher modules covering all available wiki data
- Image downloader with batch URL resolution
- Coordinator running categories in FK-safe order with progress reporting
- Tauri command wired to React with progress events
- Full test coverage on Lua-to-DB mapping logic

**Next phase:** Phase 3 (Game Engine) will implement quiz generation using this data.
