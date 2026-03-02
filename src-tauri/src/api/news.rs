use crate::models::article::Article;
use feed_rs::parser;
use futures::stream::{self, StreamExt};
use reqwest::Client;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

const GDELT_API_URL: &str = "https://api.gdeltproject.org/api/v2/doc/doc";
const NEWSAPI_URL: &str = "https://newsapi.org/v2/everything";
const DEFAULT_GDELT_MAX: usize = 100;
const DEFAULT_NEWSAPI_MAX: usize = 100;
const DEFAULT_NEWSAPI_PAGES: usize = 5;
const MAX_RSS_FEEDS_CONCURRENT: usize = 10;
const RSS_FEEDS: &[&str] = &[
    // Major News
    "https://feeds.bbci.co.uk/news/world/rss.xml",
    "https://feeds.bbci.co.uk/news/business/rss.xml",
    "https://feeds.bbci.co.uk/news/politics/rss.xml",
    "https://rss.nytimes.com/services/xml/rss/nyt/HomePage.xml",
    "https://rss.nytimes.com/services/xml/rss/nyt/World.xml",
    "https://rss.nytimes.com/services/xml/rss/nyt/Business.xml",
    "https://rss.nytimes.com/services/xml/rss/nyt/Politics.xml",
    "https://www.theguardian.com/world/rss",
    "https://www.theguardian.com/business/rss",
    "https://www.theguardian.com/politics/rss",
    "https://feeds.npr.org/1001/rss.xml",
    "https://feeds.npr.org/1006/rss.xml",
    "https://www.cnbc.com/id/100003114/device/rss/rss.html",
    "https://www.cnbc.com/id/100727362/device/rss/rss.html",
    "https://www.aljazeera.com/xml/rss/all.xml",
    // Financial/Business
    "https://feeds.reuters.com/reuters/businessnews",
    "https://feeds.reuters.com/reuters/topNews",
    "https://feeds.reuters.com/reuters/politicsnews",
    "https://feeds.bloomberg.com/markets/news.rss",
    "https://feeds.bloomberg.com/economics/news.rss",
    "https://feeds.bloomberg.com/politics/news.rss",
    "https://feeds.marketwatch.com/marketwatch/topstories",
    "https://feeds.marketwatch.com/marketwatch/politics",
    "https://feeds.wsj.com/wsj/markets",
    "https://feeds.wsj.com/wsj/business",
    "https://feeds.wsj.com/wsj/politics",
    "https://feeds.ft.com/ft/news",
    "https://feeds.ft.com/ft/world",
    "https://www.economist.com/latest/rss.xml",
    "https://www.economist.com/finance-and-economics/rss.xml",
    "https://www.economist.com/united-states/rss.xml",
    "https://feeds.afr.com/rss/afr.xml",
    // Tech/AI
    "https://techcrunch.com/feed/",
    "https://www.theverge.com/rss/index.xml",
    "https://arstechnica.com/feed/",
    "https://www.wired.com/feed/rss",
    "https://www.technologyreview.com/feed/",
    "https://www.ai-journal.com/rss.xml",
    // Sports
    "https://www.espn.com/espn/rss/news",
    "https://www.si.com/rss/si_topstories.rss",
    // Crypto
    "https://cointelegraph.com/rss",
    "https://coindesk.com/arc/outboundfeeds/rss/",
    "https://decrypt.co/feed",
    // Politics/Elections
    "https://fivethirtyeight.com/politics/feed/",
    "https://www.politico.com/rss/politics.xml",
    "https://thehill.com/rss/syndicator/19109",
    "https://www.washingtonpost.com/politics/rss_headlines.xml",
    "https://apnews.com/rss/politics",
    "https://apnews.com/rss/world-news",
    "https://apnews.com/rss/business",
];
const EXTERNAL_SIGNAL_TIMEOUT_SECS: u64 = 6;

fn load_newsapi_key() -> Option<String> {
    if let Ok(key) = std::env::var("NEWSAPI_KEY") {
        if !key.is_empty() {
            return Some(key);
        }
    }

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let env_path = std::path::Path::new(manifest_dir).join(".env");
    if dotenvy::from_path(&env_path).is_ok() {
        if let Ok(key) = std::env::var("NEWSAPI_KEY") {
            if !key.is_empty() {
                return Some(key);
            }
        }
    }

    None
}

fn hash_to_u64(value: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn normalize(text: &str) -> String {
    text.to_lowercase()
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 3)
        .map(|s| s.to_lowercase())
        .collect()
}

fn stopwords() -> HashSet<&'static str> {
    [
        "the", "and", "with", "from", "this", "that", "have", "your", "will", "about", "what", "when",
        "where", "which", "their", "there", "been", "than", "then", "them", "into", "over", "after", "before",
        "more", "most", "some", "could", "would", "should", "are", "was", "were", "for", "you", "not", "but",
        "its", "how", "why", "who", "new", "out", "via", "off", "his", "her", "our", "they", "them", "than",
        "per", "just", "said", "says", "get", "gets", "got", "now", "today", "year", "years",
    ]
    .into_iter()
    .collect()
}

fn top_keywords(titles: &[String], max_terms: usize) -> Vec<String> {
    let mut freq: HashMap<String, usize> = HashMap::new();
    let stop = stopwords();

    for title in titles {
        for token in tokenize(title) {
            if stop.contains(token.as_str()) {
                continue;
            }
            *freq.entry(token).or_insert(0) += 1;
        }
    }

    let mut terms: Vec<(String, usize)> = freq.into_iter().collect();
    terms.sort_by(|a, b| b.1.cmp(&a.1));
    terms.into_iter().take(max_terms).map(|(t, _)| t).collect()
}

fn build_article(id_seed: &str, title: &str, url: Option<String>, source: &str, published_at: Option<String>, summary: Option<String>) -> Article {
    Article {
        id: hash_to_u64(id_seed),
        title: title.to_string(),
        url,
        score: 0,
        descendants: 0,
        item_type: "story".to_string(),
        source: Some(source.to_string()),
        published_at,
        summary,
    }
}

async fn fetch_external_signals(client: &Client) -> Result<Vec<Article>, String> {
    let mut results = Vec::new();
    let econ_url = std::env::var("ECON_CALENDAR_URL").ok();
    let polls_url = std::env::var("POLL_FEED_URL").ok();

    let urls = [econ_url, polls_url]
        .into_iter()
        .flatten()
        .collect::<Vec<String>>();

    for url in urls {
        let response = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(EXTERNAL_SIGNAL_TIMEOUT_SECS))
            .header("User-Agent", "Kairos/0.1 (contact: dev)")
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            println!("External signal error: status={} url={} body={}", status, url, body);
            continue;
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let bytes = response.bytes().await.map_err(|e| e.to_string())?;
        if content_type.contains("xml") || url.ends_with(".xml") || url.ends_with(".rss") {
            let feed = parser::parse(&bytes[..]).map_err(|e| format!("RSS parse error: {} url={}", e, url))?;
            let source = feed
                .title
                .map(|t| t.content)
                .unwrap_or_else(|| url.clone());
            for entry in feed.entries {
                let title = entry.title.map(|t| t.content).unwrap_or_default();
                let link = entry.links.first().map(|l| l.href.clone());
                if title.is_empty() || link.is_none() {
                    continue;
                }
                let published_at = entry
                    .published
                    .or(entry.updated)
                    .map(|dt| dt.to_rfc3339());
                let summary = entry
                    .summary
                    .map(|s| s.content)
                    .or_else(|| entry.content.and_then(|c| c.body));
                let link_value = link.as_ref().unwrap().clone();
                results.push(build_article(&link_value, &title, link, &source, published_at, summary));
            }
            continue;
        }

        let json: Value = serde_json::from_slice(&bytes)
            .map_err(|e| format!("External signal JSON decode error: {} url={}", e, url))?;
        let items = if let Some(arr) = json.as_array() {
            arr.clone()
        } else if let Some(arr) = json.get("items").and_then(|v| v.as_array()) {
            arr.clone()
        } else {
            Vec::new()
        };

        for item in items {
            let title = item.get("title").and_then(|v| v.as_str()).unwrap_or_default();
            let link = item.get("url").and_then(|v| v.as_str()).map(|s| s.to_string());
            if title.is_empty() || link.is_none() {
                continue;
            }
            let source = item
                .get("source")
                .and_then(|v| v.as_str())
                .unwrap_or("External Signal");
            let published_at = item
                .get("published_at")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let summary = item
                .get("summary")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let link_value = link.as_ref().unwrap().clone();
            results.push(build_article(&link_value, title, link, source, published_at, summary));
        }
    }

    Ok(results)
}

async fn fetch_gdelt_articles(client: &Client, seed_terms: &[String]) -> Result<Vec<Article>, String> {
    let query = seed_terms
        .iter()
        .find(|term| term.len() >= 5)
        .cloned()
        .unwrap_or_else(|| "market".to_string());

    let response = client
        .get(GDELT_API_URL)
        .query(&[
            ("query", query.as_str()),
            ("mode", "artlist"),
            ("format", "json"),
            ("maxrecords", &DEFAULT_GDELT_MAX.to_string()),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("GDELT status={} body={}", status, body));
    }

    let body = response.text().await.map_err(|e| e.to_string())?;
    let response: Value = serde_json::from_str(&body)
        .map_err(|e| format!("GDELT decode error: {} body={}", e, body))?;

    let articles = response
        .get("articles")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut results = Vec::new();

    for item in articles {
        let title = item.get("title").and_then(|v| v.as_str()).unwrap_or_default();
        let url = item.get("url").and_then(|v| v.as_str()).map(|s| s.to_string());
        if title.is_empty() || url.is_none() {
            continue;
        }

        let published_at = item
            .get("seendate")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let source = item
            .get("sourceCountry")
            .and_then(|v| v.as_str())
            .map(|s| format!("GDELT ({})", s))
            .unwrap_or_else(|| "GDELT".to_string());

        let url_value = url.as_ref().unwrap().clone();
        results.push(build_article(&url_value, title, url, &source, published_at, None));
    }

    Ok(results)
}

async fn fetch_newsapi_articles(client: &Client, seed_terms: &[String]) -> Result<Vec<Article>, String> {
    let api_key = match load_newsapi_key() {
        Some(key) => key,
        None => {
            println!("NEWSAPI_KEY not set; skipping NewsAPI.");
            return Ok(Vec::new());
        }
    };

    let seed_query = seed_terms
        .iter()
        .take(6)
        .map(|t| format!("\"{}\"", t))
        .collect::<Vec<String>>()
        .join(" OR ");
    let generic_query = "markets OR election OR earnings OR inflation OR crypto OR politics OR sports OR rates";
    let query = if seed_query.is_empty() {
        generic_query.to_string()
    } else {
        format!("({}) OR ({})", seed_query, generic_query)
    };

    let mut results = Vec::new();

    for page in 1..=DEFAULT_NEWSAPI_PAGES {
        let response = client
            .get(NEWSAPI_URL)
            .query(&[
                ("q", query.as_str()),
                ("language", "en"),
                ("pageSize", &DEFAULT_NEWSAPI_MAX.to_string()),
                ("page", &page.to_string()),
                ("sortBy", "publishedAt"),
                ("apiKey", api_key.as_str()),
            ])
            .header("User-Agent", "Kairos/0.1 (contact: dev)")
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            println!("NewsAPI error: status={} body={}", status, body);
            break;
        }

        let response: Value = response.json().await.map_err(|e| e.to_string())?;
        if response.get("status").and_then(|v| v.as_str()) != Some("ok") {
            let message = response.get("message").and_then(|v| v.as_str()).unwrap_or("unknown error");
            println!("NewsAPI error: {}", message);
            break;
        }

        let articles = response
            .get("articles")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if articles.is_empty() {
            break;
        }

        for item in articles {
            let title = item.get("title").and_then(|v| v.as_str()).unwrap_or_default();
            let url = item.get("url").and_then(|v| v.as_str()).map(|s| s.to_string());
            if title.is_empty() || url.is_none() {
                continue;
            }

            let source = item
                .get("source")
                .and_then(|s| s.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("NewsAPI");

            let published_at = item
                .get("publishedAt")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let summary = item
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let url_value = url.as_ref().unwrap().clone();
            results.push(build_article(&url_value, title, url, source, published_at, summary));
        }
    }

    if results.is_empty() {
        println!("NewsAPI returned 0 articles for query: {}", query);
    } else {
        println!("NewsAPI fetched {} articles.", results.len());
    }

    Ok(results)
}

async fn fetch_rss_feed(client: &Client, feed_url: &str) -> Result<Vec<Article>, String> {
    let response = client
        .get(feed_url)
        .header("User-Agent", "Kairos/0.1 (contact: dev)")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("RSS status={} url={} body={}", status, feed_url, body));
    }

    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    let feed = parser::parse(&bytes[..]).map_err(|e| format!("RSS parse error: {} url={}", e, feed_url))?;

    let source = feed
        .title
        .map(|t| t.content)
        .unwrap_or_else(|| feed_url.to_string());

    let mut results = Vec::new();
    for entry in feed.entries {
        let title = entry.title.map(|t| t.content).unwrap_or_default();
        let url = entry.links.first().map(|l| l.href.clone());
        if title.is_empty() || url.is_none() {
            continue;
        }

        let published_at = entry
            .published
            .or(entry.updated)
            .map(|dt| dt.to_rfc3339());

        let summary = entry
            .summary
            .map(|s| s.content)
            .or_else(|| entry.content.and_then(|c| c.body));

        let url_value = url.as_ref().unwrap().clone();
        results.push(build_article(&url_value, &title, url, &source, published_at, summary));
    }

    Ok(results)
}

async fn fetch_rss_articles(client: &Client) -> Result<Vec<Article>, String> {
    let stream = stream::iter(RSS_FEEDS.iter().copied())
        .map(|feed_url| fetch_rss_feed(client, feed_url))
        .buffer_unordered(MAX_RSS_FEEDS_CONCURRENT);

    let mut results = Vec::new();
    let mut errors = 0;
    let mut success = 0;

    stream
        .for_each(|res| {
            match res {
                Ok(mut items) => {
                    results.append(&mut items);
                    success += 1;
                }
                Err(err) => {
                    errors += 1;
                    println!("RSS error: {}", err);
                }
            }
            futures::future::ready(())
        })
        .await;

    println!("RSS feeds: {} successful, {} errors", success, errors);

    Ok(results)
}

fn dedupe_articles(mut articles: Vec<Article>) -> Vec<Article> {
    let mut seen: HashSet<String> = HashSet::new();
    articles.retain(|article| {
        let key = article
            .url
            .as_ref()
            .map(|u| normalize(u))
            .unwrap_or_else(|| normalize(&article.title));
        seen.insert(key)
    });
    articles
}

pub async fn fetch_all_articles(seed_from_hn: bool) -> Result<Vec<Article>, String> {
    let client = Client::new();

    let hn_articles = if seed_from_hn {
        crate::api::hackernews::fetch_top_stories()
            .await
            .map_err(|e| e.to_string())?
    } else {
        Vec::new()
    };

    let seed_terms = if seed_from_hn {
        let titles: Vec<String> = hn_articles.iter().map(|a| a.title.clone()).collect();
        top_keywords(&titles, 12)
    } else {
        Vec::new()
    };

    let (gdelt_res, newsapi_res, rss_res, signals_res) = tokio::join!(
        fetch_gdelt_articles(&client, &seed_terms),
        fetch_newsapi_articles(&client, &seed_terms),
        fetch_rss_articles(&client),
        fetch_external_signals(&client)
    );

    let mut merged = hn_articles;
    match gdelt_res {
        Ok(mut gdelt) => merged.append(&mut gdelt),
        Err(err) => println!("GDELT error: {}", err),
    }
    match newsapi_res {
        Ok(mut newsapi) => merged.append(&mut newsapi),
        Err(err) => println!("NewsAPI error: {}", err),
    }
    match rss_res {
        Ok(mut rss) => merged.append(&mut rss),
        Err(err) => println!("RSS error: {}", err),
    }
    match signals_res {
        Ok(mut signals) => merged.append(&mut signals),
        Err(err) => println!("External signals error: {}", err),
    }

    let deduped = dedupe_articles(merged);
    let mut counts: HashMap<String, usize> = HashMap::new();
    for article in &deduped {
        let key = article.source.clone().unwrap_or_else(|| "Unknown".to_string());
        *counts.entry(key).or_insert(0) += 1;
    }
    println!("Article sources summary: {:?}", counts);

    Ok(deduped)
}

pub fn extract_keywords_from_titles(titles: &[String], max_terms: usize) -> Vec<String> {
    top_keywords(titles, max_terms)
}
