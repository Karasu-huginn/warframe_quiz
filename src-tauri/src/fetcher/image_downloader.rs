use std::path::Path;
use std::collections::HashMap;
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

    // Deduplicate filenames
    let unique_filenames: Vec<String> = {
        let mut seen = std::collections::HashSet::new();
        tasks.iter()
            .filter(|t| seen.insert(t.wiki_filename.clone()))
            .map(|t| t.wiki_filename.clone())
            .collect()
    };

    let url_map: HashMap<String, String> = match wiki.resolve_image_urls(&unique_filenames) {
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
