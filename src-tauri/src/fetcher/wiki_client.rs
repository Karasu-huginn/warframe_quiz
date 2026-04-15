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

        page["revisions"][0]["slots"]["main"]["*"]
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
