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
