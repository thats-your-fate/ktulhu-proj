// src/scraper/unix_scraper.rs
use serde::{Deserialize, Serialize};
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{info, error};
use std::time::Duration;
use futures::future::join_all;
#[derive(Deserialize)]
pub struct ScraperRequest {
    pub query: String,
}

#[derive(Serialize)]
pub struct ScraperResponse {
    pub query: String,
    pub results: Vec<ScraperResult>,
}

#[derive(Serialize)]
pub struct ScraperResult {
    pub source: String,
    pub title: String,
    pub url: String,
    pub paragraphs: Vec<String>,
}

pub async fn spawn_scraper_process(
    socket_path: String,
    brave_key: String,
) -> anyhow::Result<()> {

    // Remove old socket
    let _ = std::fs::remove_file(&socket_path);

    let listener = UnixListener::bind(&socket_path)?;
    info!("🕸️ Web scraper listening on {}", socket_path);

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    tokio::spawn(handle_conn(stream, client.clone(), brave_key.clone()));
                }
                Err(e) => error!("Scraper accept error: {}", e),
            }
        }
    });

    Ok(())
}

async fn handle_conn(
    mut stream: UnixStream,
    client: Client,
    api_key: String,
) {
    let mut buf = Vec::new();

    if stream.read_to_end(&mut buf).await.is_err() {
        return;
    }

    let req: ScraperRequest = match serde_json::from_slice(&buf) {
        Ok(v) => v,
        Err(_) => return,
    };

    let resp = process_query(req.query, client, api_key).await;

    let _ = stream.write_all(&serde_json::to_vec(&resp).unwrap()).await;
}

async fn process_query(
    query: String,
    client: Client,
    api_key: String
) -> ScraperResponse {

    let brave_url = format!(
        "https://api.search.brave.com/res/v1/web/search?q={}",
        urlencoding::encode(&query)
    );

    let mut out = vec![];

    let res = client
        .get(brave_url)
        .header("Accept", "application/json")
        .header("X-Subscription-Token", api_key)
        .send()
        .await;


if let Ok(br) = res {
    if let Ok(json) = br.json::<serde_json::Value>().await {
        if let Some(arr) = json["web"]["results"].as_array() {
            // ==== Build async tasks ====
            let tasks = arr.iter().take(5).map(|item| {
                let client = client.clone();

                let url = item["url"].as_str().unwrap_or("").to_string();
                let title = item["title"].as_str().unwrap_or("").to_string();
                let snippet = item["description"].as_str().unwrap_or("").to_string();

                tokio::spawn(async move {
                    // Try fetching paragraphs concurrently
                    let paras = extract_paragraphs(&url, &client)
                        .await
                        .unwrap_or_else(|| {
                            if snippet.is_empty() {
                                vec![]
                            } else {
                                vec![snippet]
                            }
                        });

                    // Skip empty results
                    if paras.is_empty() {
                        return None;
                    }

                    Some(ScraperResult {
                        source: "brave".into(),
                        title,
                        url,
                        paragraphs: paras,
                    })
                })
            });

            // ==== Join all tasks ====
            let results = join_all(tasks).await;

            // ==== Collect successful results ====
            for r in results {
                if let Ok(Some(result)) = r {
                    out.push(result);
                }
            }
        }
    }
}

    ScraperResponse {
        query,
        results: out,
    }
}

async fn extract_paragraphs(
    url: &str,
    client: &Client,
) -> Option<Vec<String>> {

    // Fetch only first 32 KB of HTML (super fast)
    let html = client
        .get(url)
        .header("Range", "bytes=0-32000")
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;

    let doc = Html::parse_document(&html);
    let selector = Selector::parse("p").unwrap();

    let mut out = vec![];
    let mut idx = 0; // paragraph counter

    for p in doc.select(&selector) {
        let text = p.text().collect::<Vec<_>>().join(" ");
        let mut clean = text.trim().replace("\n", " ").replace("\t", " ");

        // Hard skip the first 2 paragraphs (usually garbage)
        if idx < 2 {
            idx += 1;
            continue;
        }
        idx += 1;

        // Remove multiple spaces
        clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");

        // Skip junk paragraphs
        let lower = clean.to_lowercase();
        if lower.contains("enable js")
            || lower.contains("ad blocker")
            || lower.contains("cookies")
            || lower.contains("subscribe")
            || lower.contains("©")
            || lower.contains("copyright")
            || lower.contains("advertisement")
            || lower.contains("newsletter")
            || lower.contains("audience measurement")
        {
            continue;
        }

        // Only keep textual paragraphs
        if clean.len() < 40 || clean.len() > 1000 {
            continue;
        }

        out.push(clean);

        if out.len() >= 5 {
            break;
        }
    }

    if out.is_empty() { None } else { Some(out) }
}

