use rusqlite::Connection;

pub fn create_tables(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch("
        -- Tier 1: Core game entities

        CREATE TABLE IF NOT EXISTS warframes (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT NOT NULL,
            type            TEXT NOT NULL,
            description     TEXT NOT NULL DEFAULT '',
            health          REAL,
            shields         REAL,
            armor           REAL,
            energy          REAL,
            sprint_speed    REAL,
            passive         TEXT NOT NULL DEFAULT '',
            mastery_rank    INTEGER NOT NULL DEFAULT 0,
            acquisition     TEXT NOT NULL DEFAULT '',
            release_date    TEXT NOT NULL DEFAULT '',
            prime_variant   TEXT NOT NULL DEFAULT '',
            is_vaulted      INTEGER NOT NULL DEFAULT 0,
            helminth_ability TEXT NOT NULL DEFAULT '',
            sex             TEXT NOT NULL DEFAULT '',
            icon_path       TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS abilities (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT NOT NULL,
            cost            INTEGER NOT NULL DEFAULT 0,
            description     TEXT NOT NULL DEFAULT '',
            icon_path       TEXT NOT NULL DEFAULT '',
            warframe_id     INTEGER REFERENCES warframes(id),
            slot_index      INTEGER NOT NULL DEFAULT 0,
            is_helminth     INTEGER NOT NULL DEFAULT 0,
            augment_mod_name TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS weapons (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT NOT NULL,
            type            TEXT NOT NULL,
            subtype         TEXT NOT NULL DEFAULT '',
            mastery_rank    INTEGER NOT NULL DEFAULT 0,
            damage_total    REAL,
            damage_impact   REAL,
            damage_puncture REAL,
            damage_slash    REAL,
            crit_chance     REAL,
            crit_multiplier REAL,
            status_chance   REAL,
            fire_rate       REAL,
            magazine_size   INTEGER NOT NULL DEFAULT 0,
            reload_time     REAL,
            trigger_type    TEXT NOT NULL DEFAULT '',
            noise_level     TEXT NOT NULL DEFAULT '',
            riven_disposition INTEGER NOT NULL DEFAULT 0,
            acquisition     TEXT NOT NULL DEFAULT '',
            variant_type    TEXT NOT NULL DEFAULT '',
            base_weapon_id  INTEGER REFERENCES weapons(id),
            release_date    TEXT NOT NULL DEFAULT '',
            icon_path       TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS mods (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT NOT NULL,
            polarity        TEXT NOT NULL DEFAULT '',
            rarity          TEXT NOT NULL DEFAULT '',
            mod_type        TEXT NOT NULL DEFAULT '',
            max_rank        INTEGER NOT NULL DEFAULT 0,
            base_drain      INTEGER NOT NULL DEFAULT 0,
            effect_description TEXT NOT NULL DEFAULT '',
            set_name        TEXT NOT NULL DEFAULT '',
            is_exilus       INTEGER NOT NULL DEFAULT 0,
            is_augment      INTEGER NOT NULL DEFAULT 0,
            augment_warframe_id INTEGER REFERENCES warframes(id),
            icon_path       TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS characters (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            faction     TEXT NOT NULL DEFAULT '',
            location    TEXT NOT NULL DEFAULT '',
            role        TEXT NOT NULL DEFAULT '',
            voice_actor TEXT NOT NULL DEFAULT '',
            icon_path   TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS quotes (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            character_id INTEGER REFERENCES characters(id),
            quote_text   TEXT NOT NULL,
            audio_path   TEXT NOT NULL DEFAULT '',
            context      TEXT NOT NULL DEFAULT ''
        );

        -- Tier 2: World & progression

        CREATE TABLE IF NOT EXISTS bosses (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            name          TEXT NOT NULL,
            planet        TEXT NOT NULL DEFAULT '',
            faction       TEXT NOT NULL DEFAULT '',
            description   TEXT NOT NULL DEFAULT '',
            warframe_drop TEXT NOT NULL DEFAULT '',
            mechanics     TEXT NOT NULL DEFAULT '',
            icon_path     TEXT NOT NULL DEFAULT '',
            character_id  INTEGER REFERENCES characters(id)
        );

        CREATE TABLE IF NOT EXISTS companions (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            class       TEXT NOT NULL DEFAULT '',
            breed       TEXT NOT NULL DEFAULT '',
            health      REAL,
            shields     REAL,
            armor       REAL,
            description TEXT NOT NULL DEFAULT '',
            acquisition TEXT NOT NULL DEFAULT '',
            icon_path   TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS companion_precepts (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            name         TEXT NOT NULL,
            description  TEXT NOT NULL DEFAULT '',
            companion_id INTEGER REFERENCES companions(id),
            icon_path    TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS quests (
            id                    INTEGER PRIMARY KEY AUTOINCREMENT,
            name                  TEXT NOT NULL,
            description           TEXT NOT NULL DEFAULT '',
            prerequisite_quest_id INTEGER REFERENCES quests(id),
            mastery_requirement   INTEGER NOT NULL DEFAULT 0,
            reward_summary        TEXT NOT NULL DEFAULT '',
            storyline_summary     TEXT NOT NULL DEFAULT '',
            sort_order            INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS planets (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT NOT NULL,
            faction         TEXT NOT NULL DEFAULT '',
            open_world_name TEXT NOT NULL DEFAULT '',
            hub_name        TEXT NOT NULL DEFAULT '',
            boss_id         INTEGER REFERENCES bosses(id),
            tileset         TEXT NOT NULL DEFAULT '',
            icon_path       TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS planet_resources (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            planet_id     INTEGER REFERENCES planets(id),
            resource_name TEXT NOT NULL,
            rarity        TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS syndicates (
            id                   INTEGER PRIMARY KEY AUTOINCREMENT,
            name                 TEXT NOT NULL,
            description          TEXT NOT NULL DEFAULT '',
            leader_name          TEXT NOT NULL DEFAULT '',
            sigil_path           TEXT NOT NULL DEFAULT '',
            leader_character_id  INTEGER REFERENCES characters(id)
        );

        CREATE TABLE IF NOT EXISTS syndicate_relations (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            syndicate_id        INTEGER REFERENCES syndicates(id),
            related_syndicate_id INTEGER REFERENCES syndicates(id),
            relation_type       TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS relics (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            name       TEXT NOT NULL,
            era        TEXT NOT NULL DEFAULT '',
            is_vaulted INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS relic_rewards (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            relic_id   INTEGER REFERENCES relics(id),
            item_name  TEXT NOT NULL,
            item_type  TEXT NOT NULL DEFAULT '',
            rarity     TEXT NOT NULL DEFAULT ''
        );

        -- Tier 3: Deep mechanics

        CREATE TABLE IF NOT EXISTS elements (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            name         TEXT NOT NULL,
            element_type TEXT NOT NULL DEFAULT '',
            status_effect TEXT NOT NULL DEFAULT '',
            component_a  TEXT NOT NULL DEFAULT '',
            component_b  TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS faction_weaknesses (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            faction      TEXT NOT NULL,
            armor_type   TEXT NOT NULL DEFAULT '',
            weak_element TEXT NOT NULL DEFAULT '',
            strong_element TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS arcanes (
            id               INTEGER PRIMARY KEY AUTOINCREMENT,
            name             TEXT NOT NULL,
            trigger_condition TEXT NOT NULL DEFAULT '',
            effect           TEXT NOT NULL DEFAULT '',
            max_rank         INTEGER NOT NULL DEFAULT 0,
            source           TEXT NOT NULL DEFAULT '',
            equipment_type   TEXT NOT NULL DEFAULT '',
            icon_path        TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS focus_schools (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            symbol_path TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS focus_abilities (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            school_id   INTEGER REFERENCES focus_schools(id),
            is_waybound INTEGER NOT NULL DEFAULT 0,
            is_passive  INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS progenitor_elements (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            warframe_id INTEGER REFERENCES warframes(id),
            element     TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS requiem_mods (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            symbol_path TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS incarnon_weapons (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            weapon_id           INTEGER REFERENCES weapons(id),
            trigger_description TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS incarnon_evolutions (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            incarnon_weapon_id  INTEGER REFERENCES incarnon_weapons(id),
            tier                INTEGER NOT NULL DEFAULT 0,
            choice_index        INTEGER NOT NULL DEFAULT 0,
            description         TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS railjack_intrinsics (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            tree_name TEXT NOT NULL,
            rank      INTEGER NOT NULL DEFAULT 0,
            description TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS landing_craft (
            id                 INTEGER PRIMARY KEY AUTOINCREMENT,
            name               TEXT NOT NULL,
            air_support_ability TEXT NOT NULL DEFAULT '',
            description        TEXT NOT NULL DEFAULT '',
            icon_path          TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS cosmetics (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            type        TEXT NOT NULL DEFAULT '',
            warframe_id INTEGER REFERENCES warframes(id),
            acquisition TEXT NOT NULL DEFAULT '',
            icon_path   TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS lore_fragments (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            name       TEXT NOT NULL,
            type       TEXT NOT NULL DEFAULT '',
            content    TEXT NOT NULL DEFAULT '',
            audio_path TEXT NOT NULL DEFAULT '',
            icon_path  TEXT NOT NULL DEFAULT ''
        );

        -- Game tracking

        CREATE TABLE IF NOT EXISTS quiz_sessions (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            started_at      TEXT NOT NULL DEFAULT '',
            mode            TEXT NOT NULL DEFAULT '',
            score           INTEGER NOT NULL DEFAULT 0,
            total_questions INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS quiz_answers (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id      INTEGER REFERENCES quiz_sessions(id),
            category        TEXT NOT NULL DEFAULT '',
            correct_item_id INTEGER NOT NULL DEFAULT 0,
            chosen_item_id  INTEGER NOT NULL DEFAULT 0,
            is_correct      INTEGER NOT NULL DEFAULT 0,
            answered_at     TEXT NOT NULL DEFAULT ''
        );

        -- Asset cache

        CREATE TABLE IF NOT EXISTS asset_cache (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            source_url TEXT NOT NULL UNIQUE,
            local_path TEXT NOT NULL DEFAULT '',
            category   TEXT NOT NULL DEFAULT '',
            fetched_at TEXT NOT NULL DEFAULT ''
        );
    ")
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
