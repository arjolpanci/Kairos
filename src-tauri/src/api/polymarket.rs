use crate::models::market::Market;
use reqwest::Client;
use std::collections::HashSet;

const POLYMARKET_API_URL: &str = "https://gamma-api.polymarket.com/markets";
const MARKET_PAGE_LIMIT: usize = 500;
const MARKET_PAGES_PER_SORT: usize = 4;

pub async fn fetch_active_markets() -> Result<Vec<Market>, String> {
    let client = Client::new();
    let mut all_markets: Vec<Market> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // Fetch from multiple sort orders to get diverse markets
    for sort in ["volume", "liquidity", "newest"] {
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
                // Filter out markets with insufficient data or liquidity
                if market.outcome_prices.len() < 2 || market.question.trim().is_empty() {
                    continue;
                }
                // Require minimum liquidity for quality markets
                let liquidity = market.liquidity.unwrap_or(0.0);
                let volume = market.volume.unwrap_or(0.0);
                if liquidity < 5000.0 && volume < 10000.0 {
                    continue;
                }
                all_markets.push(market);
            }
        }
    }

    println!("Fetched {} unique markets from Polymarket", all_markets.len());
    Ok(all_markets)
}
