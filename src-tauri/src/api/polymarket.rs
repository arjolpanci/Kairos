use crate::models::market::Market;
use reqwest::Client;
use std::collections::HashSet;

const POLYMARKET_API_URL: &str = "https://gamma-api.polymarket.com/markets";
const MARKET_PAGE_LIMIT: usize = 200;
const MARKET_PAGES_PER_SORT: usize = 5;

pub async fn fetch_active_markets() -> Result<Vec<Market>, String> {
    let client = Client::new();
    let mut all_markets: Vec<Market> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for sort in ["volume", "liquidity"] {
        for page in 0..MARKET_PAGES_PER_SORT {
            let offset = page * MARKET_PAGE_LIMIT;
            let response_text = client
                .get(POLYMARKET_API_URL)
                .query(&[
                    ("active", "true"),
                    ("closed", "false"),
                    ("limit", &MARKET_PAGE_LIMIT.to_string()),
                    ("offset", &offset.to_string()),
                    ("sort", sort),
                ])
                .send()
                .await
                .map_err(|e| e.to_string())?
                .text()
                .await
                .map_err(|e| e.to_string())?;

            let markets: Vec<Market> = serde_json::from_str(&response_text)
                .map_err(|e| e.to_string())?;

            if markets.is_empty() {
                break;
            }

            for market in markets {
                if !seen.insert(market.id.clone()) {
                    continue;
                }
                if market.outcome_prices.len() < 2 || market.question.trim().is_empty() {
                    continue;
                }
                all_markets.push(market);
            }
        }
    }

    Ok(all_markets)
}
