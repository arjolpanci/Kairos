// src-tauri/src/api/hackernews.rs
use crate::models::article::Article;
use futures::stream::{self, StreamExt};
use reqwest::Client;

// A custom error type for cleaner error handling
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),
}

impl serde::Serialize for ApiError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

const API_BASE: &str = "https://hacker-news.firebaseio.com/v0";
const TOP_STORIES_URL: &str = "/topstories.json";
const ITEM_URL_BASE: &str = "/item/";
const MAX_STORIES: usize = 200;
const CONCURRENT_REQUESTS: usize = 10;

// This helper function fetches the full details for a single article ID.
async fn fetch_article_details(client: &Client, id: u64) -> Result<Article, reqwest::Error> {
    let url = format!("{}{}{}.json", API_BASE, ITEM_URL_BASE, id);
    client.get(url).send().await?.json::<Article>().await
}

// This is the function that will be exposed to the Svelte frontend.
pub async fn fetch_top_stories() -> Result<Vec<Article>, ApiError> {
    let client = Client::new();

    // 1. Fetch the list of top story IDs
    let top_story_ids: Vec<u64> = client
        .get(format!("{}{}", API_BASE, TOP_STORIES_URL))
        .send()
        .await?
        .json()
        .await?;

    // 2. Fetch details for each story concurrently
    let articles_stream = stream::iter(top_story_ids.into_iter().take(MAX_STORIES))
        .map(|id| fetch_article_details(&client, id))
        .buffer_unordered(CONCURRENT_REQUESTS);

    // 3. Collect results, filtering out any errors during fetching
    let all_articles: Vec<Article> = articles_stream
        .filter_map(|res| async { res.ok() })
        .collect()
        .await;

    // 4. Filter for importance and relevance
    // Relaxed thresholds to widen coverage for market scanning.
    let all_articles_len = all_articles.len();
    let filtered_articles: Vec<Article> = all_articles
        .into_iter()
        .filter(|article| {
            article.score > 30
                && article.descendants > 5
                && article.item_type == "story"
        })
        .map(|mut article| {
            article.source = Some("Hacker News".to_string());
            article
        })
        .collect();

    println!("Fetched {} HN articles, filtered down to {}", all_articles_len, filtered_articles.len());

    Ok(filtered_articles)
}
