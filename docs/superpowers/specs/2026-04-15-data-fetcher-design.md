# Phase 2: Data Fetcher — Design Spec

## Overview

Build the `fetcher` module that pulls real game data from the Warframe Wiki into the local SQLite database. Uses the MediaWiki revisions API to fetch Lua data module source code, evaluates it with an embedded Lua interpreter (`mlua`), maps the results to Rust structs, and upserts into the database. Images are downloaded and cached locally during the fetch.

## Data Source

The Warframe Wiki (https://warframe.fandom.com/) stores structured game data in Lua modules (namespace 828). These are fetched via the MediaWiki revisions API:

```
GET https://warframe.fandom.com/api.php
    ?action=query
    &titles=Module:Warframes/data
    &prop=revisions&rvprop=content&rvslots=main
    &format=json
```

The response wraps the full Lua source code as a string, extracted from `pages[*].revisions[0].slots.main.content`. The Lua code is a `return { ... }` statement that `mlua` evaluates into a Lua table.

**NOT used:** Scribunto console (requires authentication), Cargo query API (not available on this wiki), raw page access (blocked by Cloudflare).

Image URLs are resolved via the imageinfo API:
```
GET https://warframe.fandom.com/api.php
    ?action=query
    &titles=File:Excalibur.png
    &prop=imageinfo&iiprop=url
    &format=json
```

## Architecture

```
User clicks "Update Data"
         │
         ▼
┌─ Coordinator ──────────────────────────────┐
│  For each category (in FK-safe order):     │
│    1. Fetch Lua source (MediaWiki API)     │
│    2. Evaluate Lua (mlua)                  │
│    3. Map to Rust structs                  │
│    4. Upsert into SQLite (in transaction)  │
│    5. Download images (wiki imageinfo API) │
│    6. Emit progress event to frontend      │
└────────────────────────────────────────────┘
```

Category-based pipeline: each data category is an independent fetcher module. The coordinator runs them in sequence. If one category fails, others still succeed.

## Tech Stack Additions

| Crate | Purpose |
|-------|---------|
| `reqwest` (blocking) | HTTP client for wiki API + image downloads |
| `mlua` (lua54, vendored) | Embedded Lua interpreter for evaluating wiki data modules |

## Schema Migration

Drop 10 tables that have no Lua data source on the wiki:

**Dropping:** `characters`, `quotes`, `quests`, `progenitor_elements`, `incarnon_weapons`, `incarnon_evolutions`, `railjack_intrinsics`, `landing_craft`, `lore_fragments`, `cosmetics`

**Remaining (22 tables):** warframes, abilities, weapons, mods, companions, companion_precepts, bosses, planets, planet_resources, syndicates, syndicate_relations, relics, relic_rewards, elements, faction_weaknesses, arcanes, focus_schools, focus_abilities, quiz_sessions, quiz_answers, asset_cache

Code to update:
- `schema.rs`: remove 10 CREATE TABLE statements, update test assertion to 22
- `models.rs`: remove `Character`, `Quote` structs
- `queries/characters.rs`, `queries/quotes.rs`: delete
- `queries/mod.rs`: remove module declarations
- `commands/mod.rs`: remove character_count and quote_count from `DbStats`

## Category Modules

12 category fetcher modules, each mapping wiki Lua fields to DB schema:

| # | Module | Wiki Source | Approx Records | Key Lua Fields |
|---|--------|------------|-----------------|----------------|
| 1 | warframes | `Module:Warframes/data` | ~110 | Name, Type, Health, Shield, Armor, Energy, Sprint, Passive, Description, Image, Sex, Introduced, Vaulted, Subsumed |
| 2 | abilities | `Module:Ability/data` | ~400 | Name, Cost, Description, Icon, PowerSuit→warframe_id FK, Key→slot_index, Subsumable→is_helminth |
| 3 | weapons | `Module:Weapons/data/*` (8 sub-modules) | ~500 | Name, Type, Class→subtype, Damage (nested), CritChance, CritMultiplier, StatusChance, FireRate, Magazine, Reload, Trigger, Disposition, Mastery, Image |
| 4 | mods | `Module:Mods/data` | ~1000+ | Name, Polarity, Rarity, Type→mod_type, MaxRank, BaseDrain, Description, IsExilus, IsAbilityAugment→is_augment, Image |
| 5 | companions | `Module:Companions/data` | ~50 | Name, Type→class, Health, Armor, Shield, Description, Image, Mastery. Precept abilities extracted into companion_precepts if present in data, otherwise populated from `Module:Mods/data` (mods with Type="Companion" and IsAbilityAugment) |
| 6 | bosses | `Module:Enemies/data/*` (filter Type=Boss) | ~20 | Name, Faction, Description, Image, Planet, Missions |
| 7 | planets | `Module:Missions/data` | ~18 planets + ~200 nodes | Planet name, Faction, nodes with Type/Tileset/Enemy/Boss, region resources |
| 8 | factions | `Module:Factions/data` | ~50 | Name, Description, Image → syndicates table. Note: syndicate_relations may not be derivable from this module — populate what's available, leave syndicate_relations empty if alliance/opposition data isn't in the Lua source |
| 9 | focus | `Module:Focus/data` | 5 schools + ~80 abilities | Name, Description, Image, IsWayBound, IsPassive, school grouping |
| 10 | arcanes | `Module:Arcane/data` | ~100 | Name, Description→effect, Criteria→trigger_condition, MaxRank, Rarity→source, Type→equipment_type, Image |
| 11 | damage_types | `Module:DamageTypes/data` | ~15 types | Name, Color, Status, Positives/Negatives → elements + faction_weaknesses |
| 12 | relics | `Module:Void/data` | ~700 | Name, Tier→era, Vaulted, Drops→relic_rewards (Item, Part, Rarity) |

### Fetch Order (FK dependencies)

1. warframes (no deps)
2. abilities (needs warframes for FK)
3. weapons (no deps)
4. mods (warframes FK optional, can be NULL)
5. companions (no deps)
6. bosses from enemies (no deps)
7. planets/missions (needs bosses for FK)
8. factions → syndicates (no deps)
9. focus (no deps)
10. arcanes (no deps)
11. damage_types → elements + faction_weaknesses (no deps)
12. relics (no deps)

## Fetch & Parse Pipeline (per category)

### Step 1: Fetch Lua Source

HTTP GET to MediaWiki revisions API. Extract Lua source from `pages[*].revisions[0].slots.main.content`.

### Step 2: Evaluate Lua with mlua

```rust
let lua = Lua::new();
let table: mlua::Table = lua.load(&lua_source).eval()?;
```

The wiki modules return a Lua table. mlua evaluates the `return { ... }` and gives a traversable table.

### Step 3: Map to Rust Structs

Each category module has a mapping function that reads Lua table fields and builds DB model structs. Missing or nil fields become `None`.

### Step 4: Upsert to SQLite

Use `INSERT OR REPLACE` keyed on `name` (UNIQUE constraint). All writes for a single category happen inside a transaction — if a write fails partway, the transaction rolls back and existing data stays intact.

### Step 5: Download Images

For each record with an Image field:
1. Batch-resolve filenames to CDN URLs via imageinfo API (up to 50 titles per request)
2. Download each image to `assets/<category>/<filename>`
3. Skip if file already exists locally
4. Store local path in `icon_path` DB field

## Image Downloads

- Images stored in `assets/` subdirectories: `assets/warframes/`, `assets/abilities/`, `assets/weapons/`, etc.
- Image filenames come from the Lua data's `Image` or `Icon` fields
- URLs resolved via `api.php?action=query&titles=File:<name>&prop=imageinfo&iiprop=url`
- Batch resolution: up to 50 image titles per API request
- Skip download if local file already exists (no re-download on subsequent fetches)
- Failed image downloads are skipped (icon_path set to NULL), don't fail the category

## Rate Limiting

- 1 HTTP request per second to the wiki API
- Batch image URL resolution (50 per request) to minimize request count
- On HTTP 429: exponential backoff, retry up to 3 times

## Progress Reporting

The fetcher emits Tauri events as each category progresses:

```
Event: "fetch_progress"
Payload: {
    category: "warframes",
    status: "fetching" | "parsing" | "downloading_images" | "done" | "error",
    current: 3,
    total: 12,
    message: "Fetching warframes..."
}
```

React listens for these events and renders progress. The user sees which categories are done, in progress, or failed.

## Error Handling

- **Network failure (wiki unreachable):** Abort early, report error, existing DB data untouched.
- **Single category failure:** Log error, skip category, continue with rest. Report partial success ("11/12 categories updated, mods failed: parse error").
- **Image download failure:** Skip image, set icon_path to NULL. Don't fail the category.
- **Rate limit (HTTP 429):** Wait and retry up to 3 times with exponential backoff.
- **Transaction safety:** All DB writes per category are in a transaction. Failure rolls back, preserving existing data.

## File Structure

```
src-tauri/src/fetcher/
├── mod.rs              — public API: fetch_all(), fetch_category()
├── coordinator.rs      — runs categories in order, manages progress events
├── wiki_client.rs      — HTTP: fetch_module_source(), resolve_image_urls(), download_image()
├── lua_parser.rs       — mlua wrapper: evaluate Lua source, extract table entries
├── categories/
│   ├── mod.rs
│   ├── warframes.rs
│   ├── abilities.rs
│   ├── weapons.rs
│   ├── mods.rs
│   ├── companions.rs
│   ├── bosses.rs       — filters enemies by Type=Boss
│   ├── planets.rs      — missions/nodes → planets + planet_resources
│   ├── factions.rs     — → syndicates table
│   ├── focus.rs        — → focus_schools + focus_abilities
│   ├── arcanes.rs
│   ├── damage_types.rs — → elements + faction_weaknesses
│   └── relics.rs       — → relics + relic_rewards
└── image_downloader.rs — batch URL resolution + download to assets/
```

### Module Responsibilities

- **wiki_client.rs:** All HTTP to the wiki. Owns the rate limiter (1 req/sec). Methods: `fetch_module_source(module_name)`, `resolve_image_urls(filenames)`, `download_image(url, local_path)`.
- **lua_parser.rs:** Wraps mlua. Method: `parse_module(source: &str) -> Result<mlua::Table>`.
- **coordinator.rs:** Runs categories in FK-safe order. Emits progress events via Tauri app handle.
- **Each category module:** Takes `&Connection`, `&LuaParser`, `&WikiClient`. Returns `Result<CategoryReport>` with counts (inserted, updated, skipped, failed).
- **image_downloader.rs:** Batch image URL resolution + sequential downloads with rate limiting.

## Tauri Command

New command `fetch_wiki_data` exposed to React:

```rust
#[tauri::command]
async fn fetch_wiki_data(db: State<'_, Database>, app: AppHandle) -> Result<FetchReport, String>
```

Calls the coordinator, which emits `fetch_progress` events. Returns a summary report when done.

## Deferred to Later Phases

- **Characters/NPCs:** No Lua data module exists. Requires wiki page scraping (different approach).
- **Quotes/voice lines:** On individual wiki character pages. Requires HTML scraping + audio downloads.
- **Quests, Kuva Liches/Sisters, Incarnon, Railjack intrinsics, Landing Craft, Lore Fragments, Cosmetics:** No Lua modules. Dropped from schema.
