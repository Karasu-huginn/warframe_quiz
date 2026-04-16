<!-- prettier-ignore -->
<div align="center">

# Warframe Quizz

A desktop quiz game inspired by [LoLdle](https://loldle.net/), built around [Warframe](https://www.warframe.com/) game data.

Test your knowledge of Warframes, weapons, abilities, mods, bosses, planets, and more through multiple-choice questions.

[Features](#features) | [Architecture](#architecture) | [Build & run](#build--run) | [Project structure](#project-structure) | [Question types](#question-types)

</div>

## Features

- **9 question types** — Identify Warframes by abilities or image, weapons by stats, mods by effect, bosses by faction, planets by resource, focus schools, and element combinations
- **Live wiki data** — Fetches up-to-date game data directly from the [Warframe Wiki](https://warframe.fandom.com/) Lua modules via the MediaWiki API, with a built-in Lua parser
- **Local database** — All data is stored in SQLite for fast offline gameplay after the initial fetch
- **Session tracking** — Score, streak, and answer history are persisted per quiz session
- **Optional timer** — Enable a per-question countdown for an extra challenge
- **French UI** — Questions and interface are presented in French

> [!NOTE]
> Warframe quizz is in early development. The game engine and data pipeline are functional, but the frontend UI is still a scaffold.

## Architecture

```
┌─────────────────────────────────────────────┐
│                  Tauri App                  │
│                                             │
│  ┌──────────────┐      ┌──────────────────┐ │
│  │   React UI   │◄────►│   Rust Backend   │ │
│  │ (TypeScript) │ IPC  │                  │ │
│  │              │      │  - Game engine   │ │
│  │              │      │  - Question gen  │ │
│  │              │      │  - Data fetcher  │ │
│  │              │      │  - SQLite access │ │
│  └──────────────┘      └───────┬──────────┘ │
│                               │             │
│                        ┌──────┴──────┐      │
│                        │   SQLite    │      │
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

| Layer | Technology |
|-------|-----------|
| Desktop shell | [Tauri v2](https://v2.tauri.app/) |
| Backend | Rust |
| Frontend | React 18 + TypeScript |
| Build tool | Vite 6 |
| Database | SQLite via [rusqlite](https://crates.io/crates/rusqlite) |
| Wiki parsing | Embedded Lua 5.4 via [mlua](https://crates.io/crates/mlua) |
| Data source | Warframe Wiki MediaWiki API |

React communicates with Rust through Tauri IPC commands (`invoke`). Rust owns all state and logic -- the frontend never touches SQLite or the wiki directly. The wiki is only contacted when the user triggers a data update, not during gameplay.

## Prerequisites

- [Node.js](https://nodejs.org/) (LTS)
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- Tauri v2 system dependencies -- see the [Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/)

## Build & run

1. **Clone the repository and install dependencies**

   ```bash
   git clone https://github.com/Karasu-huginn/Warframe_Quizz.git && cd warframe_quizz
   npm install
   ```

2. **Run in development mode**

   ```bash
   npm run tauri dev
   ```

   This starts the Vite dev server on `http://localhost:1420` and launches the Tauri window. On first run, click **Fetch Wiki Data** to populate the local SQLite database from the Warframe Wiki.

3. **Build for production**

   ```bash
   npm run tauri build
   ```

> [!TIP]
> The first data fetch downloads 12 categories (warframes, abilities, weapons, mods, companions, bosses, planets, factions, focus schools, arcanes, damage types, relics) and their associated images. This may take a few minutes depending on your connection. Subsequent launches use the cached local database.

## Project structure

```
warframe_quizz/
├── src/                    # React frontend (TypeScript)
│   ├── App.tsx             #   Main app component
│   └── main.tsx            #   Entry point
├── src-tauri/              # Rust backend (Tauri)
│   ├── Cargo.toml          #   Rust dependencies
│   ├── tauri.conf.json     #   Tauri configuration
│   └── src/
│       ├── commands/       #   Tauri IPC command handlers
│       ├── db/             #   SQLite schema, models, and queries
│       │   ├── schema.rs   #     22-table schema definition
│       │   ├── models.rs   #     Warframe, Ability, Weapon, Mod structs
│       │   └── queries/    #     Per-entity query modules
│       ├── fetcher/        #   Wiki data pipeline
│       │   ├── wiki_client.rs    # MediaWiki API client (rate-limited)
│       │   ├── lua_parser.rs     # Lua-to-JSON evaluator
│       │   ├── coordinator.rs    # Orchestrates fetch across categories
│       │   ├── image_downloader.rs
│       │   └── categories/       # Per-category fetch logic (12 categories)
│       └── game/           #   Quiz engine
│           ├── mod.rs      #     Session management, scoring, streaks
│           ├── question_types.rs  # Question, Answer, Clue types
│           └── generators/ #     9 question generators
├── index.html              # Vite entry HTML
├── package.json
├── vite.config.ts
└── tsconfig.json
```

## Question types

| Generator | Question (FR) | Clue type |
|-----------|--------------|-----------|
| `warframe_by_abilities` | "A quelle Warframe appartiennent ces capacites ?" | List of 4 ability names |
| `warframe_by_ability` | Single ability identification | Ability name |
| `warframe_by_image` | Identify Warframe from portrait | Image |
| `weapon_by_stats` | Identify weapon from stat block | Stat table |
| `mod_by_effect` | Identify mod from its effect text | Text |
| `boss_faction` | Match boss to faction | Text |
| `planet_by_resource` | Identify planet from resource list | Text list |
| `element_combination` | Which element do two base elements create? | Two elements |
| `focus_school_by_ability` | Match focus ability to school | Text |

