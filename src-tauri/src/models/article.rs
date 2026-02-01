// src-tauri/src/models/article.rs
use serde::{Serialize, Deserialize};

// This struct represents a single story from the Hacker News API.
// We derive Serialize to send it TO the frontend,
// and Deserialize to read it FROM the API response.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Article {
    pub id: u64,
    pub title: String,
    pub url: Option<String>,
    pub score: i32,
    pub descendants: i32,
    #[serde(rename = "type")]
    pub item_type: String,
}