# Warframedle — Tech Stack & Architecture Design

## Overview

Warframedle is a desktop quiz game inspired by LoLdle, built around Warframe game data. Players answer multiple-choice questions about Warframes, weapons, abilities, characters, mods, and more. The UI language is French. The game ships as a portable Windows `.exe`.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop shell | Tauri v2 |
| Backend | Rust |
| Frontend | React |
| Database | SQLite (via `rusqlite`) |
| Data source | Warframe Wiki MediaWiki API (Lua modules) |
| Packaging | Portable `.exe` (no installer) |

## Architecture

```
┌─────────────────────────────────────────────┐
│                  Tauri App                  │
│                                             │
│  ┌─────────────┐      ┌──────────────────┐  │
│  │   React UI  │◄────►│   Rust Backend   │  │
│  │             │ IPC  │                  │  │
│  │  - Screens  │      │  - Game engine   │  │
│  │  - Quiz UI  │      │  - Question gen  │  │
│  │  - Results  │      │  - Data fetcher  │  │
│  │  - Data Mgr │      │  - SQLite access │  │
│  └─────────────┘      └───────┬──────────┘  │
│                               │             │
│                        ┌──────┴──────┐      │
│                        │   SQLite    │      │
│                        │  (local DB) │      │
│                        └─────────────┘      │
└─────────────────────────────────────────────┘
                        │
                        │ HTTP (on demand)
                        ▼
               ┌─────────────────┐
               │  Warframe Wiki  │
               │  MediaWiki API  │
               └─────────────────┘
```

- React calls Rust functions through Tauri's IPC invoke system. Commands are defined in Rust (e.g., `start_quiz`, `submit_answer`, `fetch_wiki_data`) and called from JS as async functions.
- Rust owns all state and logic. React never touches SQLite or the wiki directly.
- The wiki is only contacted when the user triggers a data update, not during gameplay.

## Rust Backend Modules

### `db` — SQLite access layer
- Manages the database connection using the `rusqlite` crate.
- Provides query functions like `get_warframes(type_filter)`, `get_random_abilities(count)`, `get_character_by_id(id)`.
- Handles schema creation and migrations on first launch.

### `fetcher` — Warframe Wiki data pipeline
- Calls the MediaWiki API's Scribunto console endpoint to pull data from Lua modules (`Module:Warframes/data`, `Module:Weapons/data`, `Module:Mods/data`, etc.).
- Parses API responses into Rust structs.
- Diffs against existing DB records: inserts new, updates changed, marks removed.
- Downloads and caches icon/image/audio assets locally.

### `game` — Quiz engine
- Generates questions: picks a category, selects a random item, builds 4 answer choices (1 correct + 3 wrong from the same pool, same type).
- Tracks the current quiz session: score, question history, streaks.
- Validates answers and returns results.

### `commands` — Tauri IPC layer
- Thin layer that exposes Rust functions to the React frontend.
- Each command maps to a game/db/fetcher function (e.g., `#[tauri::command] fn start_quiz(...)`).
- No business logic — just translates between frontend calls and internal modules.

## Database Schema

### Core game data (Tier 1)

```sql
warframes       (id, name, type, description, health, shields, armor, energy,
                 sprint_speed, passive, mastery_rank, acquisition, release_date,
                 prime_variant, is_vaulted, helminth_ability, sex, icon_path)

abilities       (id, name, cost, description, icon_path, warframe_id FK,
                 slot_index, is_helminth, augment_mod_name)

weapons         (id, name, type, subtype, mastery_rank, damage_total,
                 damage_impact, damage_puncture, damage_slash,
                 crit_chance, crit_multiplier, status_chance,
                 fire_rate, magazine_size, reload_time, trigger_type,
                 noise_level, riven_disposition, acquisition,
                 variant_type, base_weapon_id FK, release_date, icon_path)

mods            (id, name, polarity, rarity, mod_type, max_rank, base_drain,
                 effect_description, set_name, is_exilus, is_augment,
                 augment_warframe_id FK, icon_path)

characters      (id, name, description, faction, location, role,
                 voice_actor, icon_path)

quotes          (id, character_id FK, quote_text, audio_path, context)
```

### Tier 2

```sql
bosses          (id, name, planet, faction, description, warframe_drop,
                 mechanics, icon_path, character_id FK)

companions      (id, name, class, breed, health, shields, armor,
                 description, acquisition, icon_path)

companion_precepts (id, name, description, companion_id FK, icon_path)

quests          (id, name, description, prerequisite_quest_id FK,
                 mastery_requirement, reward_summary, storyline_summary,
                 sort_order)

planets         (id, name, faction, open_world_name, hub_name,
                 boss_id FK, tileset, icon_path)

planet_resources (id, planet_id FK, resource_name, rarity)

syndicates      (id, name, description, leader_name, sigil_path,
                 leader_character_id FK)

syndicate_relations (id, syndicate_id FK, related_syndicate_id FK,
                     relation_type)

relics          (id, name, era, is_vaulted)

relic_rewards   (id, relic_id FK, item_name, item_type, rarity)
```

### Tier 3

```sql
elements        (id, name, element_type, status_effect,
                 component_a, component_b)

faction_weaknesses (id, faction, armor_type, weak_element, strong_element)

arcanes         (id, name, trigger_condition, effect, max_rank,
                 source, equipment_type, icon_path)

focus_schools   (id, name, description, symbol_path)

focus_abilities (id, name, description, school_id FK,
                 is_waybound, is_passive)

progenitor_elements (id, warframe_id FK, element)

requiem_mods    (id, name, symbol_path)

incarnon_weapons (id, weapon_id FK, trigger_description)

incarnon_evolutions (id, incarnon_weapon_id FK, tier, choice_index,
                     description)

railjack_intrinsics (id, tree_name, rank, description)

landing_craft   (id, name, air_support_ability, description, icon_path)

cosmetics       (id, name, type, warframe_id FK, acquisition, icon_path)

lore_fragments  (id, name, type, content, audio_path, icon_path)
```

### Game session tracking

```sql
quiz_sessions   (id, started_at, mode, score, total_questions)
                -- mode values: category name (e.g., "warframes", "abilities", "quotes")

quiz_answers    (id, session_id FK, category, correct_item_id,
                 chosen_item_id, is_correct, answered_at)
```

### Asset cache

```sql
asset_cache     (id, source_url, local_path, category, fetched_at)
```

## Data Fetcher

### Data source
The Warframe Wiki stores structured game data in Lua modules (`Module:Warframes/data`, `Module:Weapons/data`, `Module:Mods/data`, etc.). The MediaWiki API's Scribunto console endpoint lets us execute Lua code and get raw data tables back as JSON.

### Fetch pipeline per category
1. Call the API, get raw Lua table data.
2. Parse into Rust structs.
3. Diff against existing DB records (insert new, update changed, mark removed).
4. Download associated images/icons/audio to a local cache folder.

### Asset storage
- Images: `assets/warframes/`, `assets/abilities/`, `assets/weapons/`, etc.
- Audio: `assets/audio/`
- Source CDN: `static.wikia.nocookie.net`

### When it runs
- Manually triggered by the user via "Update Data" button in the UI.
- Not automatic. The user decides when to pull fresh data.
- Shows progress (e.g., "Fetching weapons... 3/7 categories done").

### Priority
Tier 1 categories first (warframes, abilities, weapons, mods, characters + quotes). Tier 2 and 3 added incrementally.

## React Frontend

### Screens

- **Home** — Pick a quiz category or game mode.
- **Quiz** — Core gameplay: see a clue (image, text, stat, audio player), pick from 4 answers, get instant feedback, next question.
- **Results** — End-of-session summary: score, streaks, time taken.
- **Data Manager** — Trigger wiki data fetch, see last update date, view download progress.
- **Stats** — Historical performance from quiz session data.

### UI flow

```
Home → select category → Quiz (N questions) → Results
                                    ↕
                              answer feedback
Home → Data Manager → trigger fetch → progress bar → done
Home → Stats → view history
```

### Responsibilities
- Calls Rust commands via Tauri IPC (`invoke("start_quiz", { category, count })`, `invoke("submit_answer", { answerId })`).
- Renders whatever Rust returns: question text, image paths, answer options, results.
- Plays audio files for quote-based questions.
- Does NOT generate questions, query the DB, or contain game logic.

## Packaging & Distribution

### Build output
Tauri builds a single portable `.exe` for Windows. The SQLite database and cached assets are stored in an app data subfolder on first launch.

### First launch experience
1. User runs the `.exe`.
2. App creates an empty SQLite database with all tables.
3. App prompts the user to fetch data ("No game data found. Fetch now?").
4. Fetcher runs, populates the DB, downloads assets.
5. User can play.

### Updates
- Game data: user clicks "Update Data" in the Data Manager.
- App itself: no auto-updater. New versions are a new `.exe` download.

## Quiz Data Categories

### Tier 1 — Rich data, high quiz value
| Category | Records | Quiz modes |
|----------|---------|-----------|
| Warframes | 110+ | Classic grid, ability guess, passive guess, silhouette, Helminth matching |
| Weapons | Hundreds | Stat guess, image guess, classic grid, variant matching |
| Mods | Thousands | Icon guess, effect guess, set matching, polarity/rarity |
| Characters/NPCs | Dozens | **Quote guess (audio)**, faction guess, image guess |
| Abilities | 400+ | Icon guess, description guess, cost guess |

### Tier 2 — Good data, solid quiz potential
| Category | Records | Quiz modes |
|----------|---------|-----------|
| Bosses | 20+ | Drop matching, location guess, quote guess |
| Companions | 7 classes | Precept matching, image guess |
| Quests | 30+ | Reward matching, lore excerpts, chronology |
| Planets/Star Chart | 18+ | Resource matching, faction-planet, tileset guess |
| Syndicates | 15+ | Symbol guess, relationship matrix |
| Void Relics/Primes | Many | Prime-to-relic matching |

### Tier 3 — Niche but fun
| Category | Quiz modes |
|----------|-----------|
| Damage/Elements | Element combo quiz |
| Arcanes | Effect matching |
| Focus Schools | School-ability matching |
| Kuva Liches/Sisters | Progenitor element matching |
| Incarnon Weapons | Evolution guess |
| Railjack | Intrinsic matching |
| Cosmetics | Skin-to-frame matching |
| Lore Fragments | Art/audio identification |
| Landing Craft | Air support ability matching |

### Voice lines / audio sources
Characters, Bosses, Lotus/mission control, Nora Night, Cephalon Cy, Lich/Sister personalities, Leverian narration, Cephalon Fragments, Duviri characters, faction enemies.
