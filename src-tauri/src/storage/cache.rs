use crate::models::{article::Article, market::Market};
use rusqlite::{params, Connection};
use serde_json::Value;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub struct Cache {
    conn: Connection,
}

impl Cache {
    pub fn new(app: &AppHandle) -> Result<Self, String> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let db_path: PathBuf = dir.join("cache.sqlite");
        let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
        let cache = Self { conn };
        cache.init_schema()?;
        Ok(cache)
    }

    fn init_schema(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS articles (
                    id INTEGER PRIMARY KEY,
                    title TEXT NOT NULL,
                    url TEXT,
                    score INTEGER,
                    descendants INTEGER,
                    item_type TEXT,
                    source TEXT,
                    published_at TEXT,
                    summary TEXT,
                    updated_at INTEGER NOT NULL
                );
                CREATE UNIQUE INDEX IF NOT EXISTS idx_articles_url ON articles(url);

                CREATE TABLE IF NOT EXISTS markets (
                    id TEXT PRIMARY KEY,
                    payload TEXT NOT NULL,
                    updated_at INTEGER NOT NULL
                );
                "#,
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn upsert_articles(&mut self, articles: &[Article]) -> Result<(), String> {
        let now = chrono::Utc::now().timestamp();
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx
                .prepare(
                    r#"
                    INSERT INTO articles (
                        id, title, url, score, descendants, item_type, source, published_at, summary, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                    ON CONFLICT(id) DO UPDATE SET
                        title=excluded.title,
                        url=excluded.url,
                        score=excluded.score,
                        descendants=excluded.descendants,
                        item_type=excluded.item_type,
                        source=excluded.source,
                        published_at=excluded.published_at,
                        summary=excluded.summary,
                        updated_at=excluded.updated_at
                    "#,
                )
                .map_err(|e| e.to_string())?;
            for article in articles {
                stmt.execute(params![
                    article.id as i64,
                    article.title,
                    article.url,
                    article.score,
                    article.descendants,
                    article.item_type,
                    article.source,
                    article.published_at,
                    article.summary,
                    now
                ])
                .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_recent_articles(&self, max_age_hours: i64) -> Result<Vec<Article>, String> {
        let cutoff = chrono::Utc::now().timestamp() - max_age_hours * 3600;
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT id, title, url, score, descendants, item_type, source, published_at, summary
                FROM articles
                WHERE updated_at >= ?1
                ORDER BY updated_at DESC
                "#,
            )
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![cutoff], |row| {
                Ok(Article {
                    id: row.get::<_, i64>(0)? as u64,
                    title: row.get(1)?,
                    url: row.get(2)?,
                    score: row.get(3)?,
                    descendants: row.get(4)?,
                    item_type: row.get(5)?,
                    source: row.get(6)?,
                    published_at: row.get(7)?,
                    summary: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut articles = Vec::new();
        while let Some(row) = rows.next().transpose().map_err(|e| e.to_string())? {
            articles.push(row);
        }
        Ok(articles)
    }

    pub fn upsert_markets(&mut self, markets: &[Market]) -> Result<(), String> {
        let now = chrono::Utc::now().timestamp();
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx
                .prepare(
                    r#"
                    INSERT INTO markets (id, payload, updated_at)
                    VALUES (?1, ?2, ?3)
                    ON CONFLICT(id) DO UPDATE SET
                        payload=excluded.payload,
                        updated_at=excluded.updated_at
                    "#,
                )
                .map_err(|e| e.to_string())?;

            for market in markets {
                let payload = serde_json::to_string(market).map_err(|e| e.to_string())?;
                stmt.execute(params![market.id, payload, now])
                    .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_recent_markets(&self, max_age_hours: i64) -> Result<Vec<Market>, String> {
        let cutoff = chrono::Utc::now().timestamp() - max_age_hours * 3600;
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT payload
                FROM markets
                WHERE updated_at >= ?1
                "#,
            )
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![cutoff], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;

        let mut markets = Vec::new();
        while let Some(row) = rows.next().transpose().map_err(|e| e.to_string())? {
            let value: Value = serde_json::from_str(&row).map_err(|e| e.to_string())?;
            let market: Market = serde_json::from_value(value).map_err(|e| e.to_string())?;
            markets.push(market);
        }
        Ok(markets)
    }
}
