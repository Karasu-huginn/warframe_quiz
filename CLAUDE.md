# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Warframedle** is a desktop quiz game inspired by LoLdle, built around Warframe game data. Players answer multiple-choice questions about Warframes, abilities, and characters. The UI language is French.

The project is in early prototyping. `codetester1.py` demonstrates the quiz question generation concept but is **not** part of the target architecture — it is a throwaway prototype showing intent. The tech stack for the real application is not yet decided.

## Planned Architecture (Not Yet Built)

The game will have five major components:

1. **UI** — Desktop GUI for the quiz experience
2. **Data fetcher** — Scrapes/pulls accurate data from the [Warframe Wiki](https://warframe.fandom.com/) to populate questions and answers
3. **Backend logic** — Game loop, scoring, question generation, answer validation
4. **Database** — Persistent storage replacing the current flat-file approach
5. **Installer** — Packaging for end-user distribution

## Data Files

All data lives in flat `.txt` files using **JSON Lines** format (one JSON object per line, not a JSON array). These are the authoritative data sources until a database is introduced.

| File | Records | Schema fields |
|------|---------|---------------|
| `warframes.txt` | 63 | `Name`, `Type` (Warframe/Archwing/Necramech), `Ability_1`–`Ability_4`, `Description`, `Icon` |
| `abilities.txt` | 265 | `Name`, `Cost` (energy, int), `Description`, `Icon`, `Powersuit` |
| `characters.txt` | 161 | `Name`, `Description` (currently empty), `Faction`, `Icon` |

The `old/` directory contains superseded data snapshots — do not use for game logic.

## Assets

- Character/warframe images: `.jpg` files in the project root (only a handful exist so far)
- Ability icons: `.png` files in `img/abilities/` (only one exists so far)
- Base URL for downloading ability icons from the wiki is stored in `abilities image download link part.txt`

## Key Decisions & Constraints

- The Warframe Wiki (https://warframe.fandom.com/) is the canonical data source for all game content
- Questions are presented in French (e.g., "À quel(le) Warframe appartiennent ces capacités ?")
- Question categories: WARFRAME, ABILITY, CHARACTER (and planned: MOD)
- Each question offers 4 multiple-choice answers with one correct answer
- Wrong answers are generated from the same data pool, filtered to avoid duplicates and match the same `Type`/category as the correct answer
