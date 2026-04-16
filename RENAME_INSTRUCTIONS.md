# Post-Rename Instructions

The project was renamed from `warframedle` to `warframe_quiz` and moved to a new directory. The following changes are needed to make everything build and run correctly after the move.

## Files to update

### 1. `src-tauri/Cargo.toml`
- Change `name = "warframedle"` to `name = "warframe_quiz"`

### 2. `src-tauri/Cargo.toml` — lib section
- Change `name = "warframedle_lib"` to `name = "warframe_quiz_lib"`

### 3. `src-tauri/src/main.rs`
- Change `warframedle_lib::run()` to `warframe_quiz_lib::run()`

### 4. `src-tauri/tauri.conf.json`
- `"productName"` can stay as `"Warframedle"` (display name) or change to `"Warframe Quiz"` — your choice
- `"identifier"` — change `"com.warframedle.app"` to `"com.warframe-quiz.app"` (or similar)
- **IMPORTANT:** Changing the identifier changes the AppData directory. The existing database and downloaded assets at `%APPDATA%/com.warframedle.app/` will NOT be found. The user will need to re-fetch wiki data after this change. To preserve data, manually copy `%APPDATA%/com.warframedle.app/` to the new path.

### 5. `package.json`
- Change `"name": "warframedle"` to `"name": "warframe-quiz"`

### 6. `index.html`
- Change `<title>Warframedle</title>` to `<title>Warframe Quiz</title>` (optional — display only)

### 7. `src/components/Home.tsx`
- The logo text `Warframedle` is hardcoded in the JSX. Change to `Warframe Quiz` if desired.

### 8. `src/App.css`
- No changes needed — CSS uses variables, no hardcoded project name.

### 9. `CLAUDE.md`
- Update project name references from "Warframedle" to "Warframe Quiz" throughout.

### 10. localStorage keys
- The app uses `warframedle-theme`, `warframedle-timer`, `warframedle-timer-seconds` as localStorage keys in `src/App.tsx` and `src/components/Settings.tsx`. These are cosmetic — they can stay as-is or be renamed to `warframe-quiz-*`. If renamed, existing user settings will reset.

## After making changes

```bash
# Clean build artifacts (old crate name cached)
cd src-tauri && cargo clean

# Rebuild
cd .. && npm install && npm run build
cd src-tauri && cargo build

# Run tests
cargo test

# Run dev
cd .. && npm run tauri dev
```

## What does NOT need to change

- All Rust source files (`src-tauri/src/**/*.rs`) — no hardcoded project name in code
- All React components (except Home.tsx logo text) — no hardcoded project name
- Database schema, fetcher, game engine — all project-name-agnostic
- Git history — preserved as-is after the move
