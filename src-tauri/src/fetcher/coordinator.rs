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
