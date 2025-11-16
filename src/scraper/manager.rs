// src/scraper/manager.rs
use crate::config::{AppConfig};
use crate::scraper::unix_scraper::spawn_scraper_process;

use crate::util::process_registry::ProcessRegistry;

use tracing::{info, warn};
use std::sync::Arc;

pub async fn spawn_scrapers_from_config(
    cfg: &AppConfig,
    _registry: Arc<ProcessRegistry>
) {
    if cfg.web_scrapper.is_empty() {
        info!("🕷️ No web scrapers configured.");
        return;
    }

    let brave_key = cfg.brave
        .as_ref()
        .map(|b| b.apikey.clone())
        .unwrap_or_else(|| "".to_string());

    for scraper in &cfg.web_scrapper {
        if scraper.r#type != "unix_socket" {
            warn!("❌ Unsupported scraper type: {}", scraper.r#type);
            continue;
        }

        info!(
            "🔍 Starting scraper `{}` at {}",
            scraper.name, scraper.socket_path
        );

        // Scraper runs as a tokio task, NOT a spawned process
        spawn_scraper_process(scraper.socket_path.clone(), brave_key.clone())
            .await
            .expect("Failed to spawn web scraper");
    }
}
