use crate::api::{news::fetch_all_articles, polymarket::fetch_active_markets};
use crate::models::{article::Article, market::Market};
use async_openai::{
    types::{
        CreateChatCompletionRequestArgs, ChatCompletionRequestMessage,
        CreateEmbeddingRequestArgs,
    },
    Client as OpenAIClient,
};
use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Recommendation {
    pub market_question: String,
    pub market_slug: String,
    pub source_article_title: String,
    pub source_article_url: String,
    pub identified_edge: f64,
    pub suggested_outcome: String,
    pub market_price: f64,
    pub estimated_prob: f64,
    pub reasoning: String,
    pub risk_level: String,
    pub suggested_budget: f64,
    pub source: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct AnalysisStatus {
    step: String,
    message: String,
    progress: f32,
}

// Manual Cosine Similarity to avoid extra dependencies
fn cosine_similarity(vec_a: &[f32], vec_b: &[f32]) -> f64 {
    let dot_product: f64 = vec_a.iter().zip(vec_b).map(|(a, b)| (*a as f64) * (*b as f64)).sum();
    let norm_a: f64 = vec_a.iter().map(|a| (*a as f64).powi(2)).sum::<f64>().sqrt();
    let norm_b: f64 = vec_b.iter().map(|b| (*b as f64).powi(2)).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { return 0.0; }
    dot_product / (norm_a * norm_b)
}

fn timeframe_to_days(timeframe: &str) -> i64 {
    match timeframe {
        "24h" => 1,
        "1w" => 7,
        "1m" => 30,
        "1y" => 365,
        "all" => 30,
        _ => 30,
    }
}

fn emit_status(app: &AppHandle, step: &str, message: &str, progress: f32) {
    let _ = app.emit(
        "analysis_status",
        AnalysisStatus {
            step: step.to_string(),
            message: message.to_string(),
            progress,
        },
    );
}

fn normalize_text(value: &str) -> String {
    value.to_lowercase()
}

fn keyword_overlap_score(a: &str, b: &str) -> f64 {
    let a_tokens: std::collections::HashSet<String> = a
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 3)
        .map(|s| s.to_lowercase())
        .collect();
    let b_tokens: std::collections::HashSet<String> = b
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 3)
        .map(|s| s.to_lowercase())
        .collect();
    if a_tokens.is_empty() || b_tokens.is_empty() {
        return 0.0;
    }
    let intersect = a_tokens.intersection(&b_tokens).count() as f64;
    intersect / (a_tokens.len().max(1) as f64)
}

fn market_text(market: &Market) -> String {
    let mut parts = Vec::new();
    parts.push(market.question.clone());
    if !market.outcomes.is_empty() {
        parts.push(market.outcomes.join(" "));
    }
    if !market.events.is_empty() {
        let event_titles: Vec<String> = market
            .events
            .iter()
            .map(|e| e.title.clone())
            .collect();
        if !event_titles.is_empty() {
            parts.push(event_titles.join(" "));
        }
    }
    if !market.series.is_empty() {
        let series_titles: Vec<String> = market
            .series
            .iter()
            .map(|s| s.title.clone())
            .collect();
        if !series_titles.is_empty() {
            parts.push(series_titles.join(" "));
        }
    }
    parts.join(" | ")
}

fn market_slug_for_url(market: &Market) -> String {
    if let Some(slug) = market.slug.clone() {
        return slug;
    }
    if let Some(event) = market.events.first() {
        return event.slug.clone();
    }
    market.id.clone()
}

fn parse_datetime(value: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(Utc.from_utc_datetime(&dt));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Some(Utc.from_utc_datetime(&dt));
    }
    None
}

fn market_end_date(market: &Market) -> Option<DateTime<Utc>> {
    let mut dates: Vec<DateTime<Utc>> = Vec::new();
    if let Some(end) = &market.end_date {
        if let Some(dt) = parse_datetime(end) {
            dates.push(dt);
        }
    }
    for event in &market.events {
        if let Some(end) = &event.end_date {
            if let Some(dt) = parse_datetime(end) {
                dates.push(dt);
            }
        }
    }
    dates.into_iter().min()
}

fn filter_markets_by_timeframe<'a>(markets: &'a [Market], timeframe_days: i64) -> Vec<&'a Market> {
    let now = Utc::now();

    if timeframe_days <= 7 {
        let horizon = now + Duration::days(timeframe_days);
        return markets
            .iter()
            .filter(|market| {
                if let Some(end) = market_end_date(market) {
                    end >= now && end <= horizon
                } else {
                    false
                }
            })
            .collect();
    }

    let long_horizon = now + Duration::days(30);
    markets
        .iter()
        .filter(|market| {
            if let Some(end) = market_end_date(market) {
                end >= long_horizon
            } else {
                false
            }
        })
        .collect()
}

fn yes_price(market: &Market) -> Option<f64> {
    if market.outcomes.len() != market.outcome_prices.len() {
        return None;
    }
    let mut yes_index: Option<usize> = None;
    for (idx, outcome) in market.outcomes.iter().enumerate() {
        let normalized = outcome.trim().to_lowercase();
        if normalized == "yes" || normalized == "true" {
            yes_index = Some(idx);
            break;
        }
    }
    if let Some(idx) = yes_index {
        return market.outcome_prices.get(idx).cloned();
    }
    market.outcome_prices.get(0).cloned()
}

fn risk_level(market: &Market, edge: f64, timeframe_days: i64) -> String {
    let liquidity = market.liquidity.unwrap_or(0.0);
    let volume = market.volume.unwrap_or(0.0);
    let liquidity_norm = (liquidity / 50000.0).min(1.0);
    let volume_norm = (volume / 50000.0).min(1.0);
    let edge_norm = (edge.abs() / 0.2).min(1.0);

    let mut score = (1.0 - liquidity_norm) * 0.4 + (1.0 - volume_norm) * 0.3 + edge_norm * 0.3;
    let horizon_bias = if timeframe_days <= 1 {
        -0.05
    } else if timeframe_days <= 7 {
        0.0
    } else if timeframe_days <= 30 {
        0.05
    } else if timeframe_days <= 365 {
        0.1
    } else {
        0.15
    };
    score = (score + horizon_bias).clamp(0.0, 1.0);

    if score < 0.4 {
        "low-risk".to_string()
    } else if score < 0.7 {
        "medium-risk".to_string()
    } else {
        "high-risk".to_string()
    }
}

fn budget_weights(timeframe_days: i64) -> (f64, f64, f64) {
    if timeframe_days <= 1 {
        (0.7, 0.2, 0.1)
    } else if timeframe_days <= 7 {
        (0.6, 0.25, 0.15)
    } else if timeframe_days <= 30 {
        (0.5, 0.3, 0.2)
    } else if timeframe_days <= 365 {
        (0.4, 0.35, 0.25)
    } else {
        (0.3, 0.35, 0.35)
    }
}

fn clamp_f64(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

fn liquidity_score(liquidity: f64, volume: f64) -> f64 {
    let liq = (liquidity / 100000.0).min(1.0);
    let vol = (volume / 100000.0).min(1.0);
    (liq * 0.6 + vol * 0.4).clamp(0.0, 1.0)
}

pub async fn run_full_analysis(
    app: AppHandle,
    budget: Option<f64>,
    timeframe: Option<String>,
) -> Result<Vec<Recommendation>, String> {
    dotenvy::dotenv().ok();
    let openai_client = OpenAIClient::new();
    let budget = budget.unwrap_or(100.0).max(0.0);
    let timeframe = timeframe.unwrap_or_else(|| "all".to_string());
    let timeframe_days = timeframe_to_days(&timeframe);

    // 1. Fetch Data
    emit_status(&app, "fetch", "Fetching articles and markets...", 0.1);
    let (articles_res, markets_res) = tokio::join!(fetch_all_articles(true), fetch_active_markets());
    let articles = articles_res.map_err(|e| e.to_string())?;
    let markets = markets_res.map_err(|e| e.to_string())?;

    if articles.is_empty() || markets.is_empty() {
        return Ok(vec![]);
    }

    emit_status(
        &app,
        "filter",
        "Using all active markets (no timeframe filter)...",
        0.2,
    );

    let markets_by_timeframe: Vec<&Market> = markets.iter().collect();

    // 2. Prepare text for embedding (Take top 80 to increase coverage)
    let article_texts: Vec<String> = articles
        .iter()
        .take(80)
        .map(|a| {
            let mut text = a.title.clone();
            if let Some(summary) = &a.summary {
                text.push_str(" | ");
                text.push_str(summary);
            }
            text
        })
        .collect();

    let mut keywords: Vec<String> = crate::api::news::extract_keywords_from_titles(&article_texts, 18);
    for kw in ["election", "inflation", "fed", "rates", "crypto", "earnings", "sports", "politics", "ai", "oil"] {
        keywords.push(kw.to_string());
    }
    keywords.sort();
    keywords.dedup();
    let mut relevant_markets: Vec<&Market> = markets_by_timeframe
        .iter()
        .copied()
        .filter(|market| {
            let question = normalize_text(&market.question);
            keywords.iter().any(|kw| question.contains(kw))
        })
        .collect();
    if relevant_markets.is_empty() {
        relevant_markets = markets_by_timeframe;
    }

    emit_status(
        &app,
        "embedding",
        "Generating semantic embeddings...",
        0.35,
    );

    let market_questions: Vec<String> = relevant_markets
        .iter()
        .take(800)
        .map(|m| market_text(m))
        .collect();

    // 3. Generate Embeddings via OpenAI
    let article_emb_req = CreateEmbeddingRequestArgs::default()
        .model("text-embedding-3-small")
        .input(article_texts.clone())
        .build()
        .map_err(|e| e.to_string())?;

    let article_embeddings = openai_client.embeddings().create(article_emb_req).await.map_err(|e| e.to_string())?;

    let market_emb_req = CreateEmbeddingRequestArgs::default()
        .model("text-embedding-3-small")
        .input(market_questions)
        .build()
        .map_err(|e| e.to_string())?;

    let market_embeddings = openai_client.embeddings().create(market_emb_req).await.map_err(|e| e.to_string())?;

    // 4. Find Matches (Vector Search)
    let mut relevant_pairs: Vec<(&Article, &Market, f64)> = Vec::new();

    for (i, art_emb) in article_embeddings.data.iter().enumerate() {
        let mut scored: Vec<(&Market, f64)> = Vec::new();
        for (j, mar_emb) in market_embeddings.data.iter().enumerate() {
            let cosine = cosine_similarity(&art_emb.embedding, &mar_emb.embedding);
            if let Some(market) = relevant_markets.get(j) {
                let article_text = article_texts.get(i).unwrap_or(&articles[i].title);
                let overlap = keyword_overlap_score(article_text, &market.question);
                let score = 0.75 * cosine + 0.25 * overlap;
                if score > 0.25 {
                    scored.push((*market, score));
                }
            }
        }
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (market, score) in scored.into_iter().take(10) {
            relevant_pairs.push((&articles[i], market, score));
        }
    }

    // 5. Analyze Matches with LLM
    let mut llm_candidates: Vec<Recommendation> = Vec::new();

    // Limit to top 8 analysis calls to be fast
    relevant_pairs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    if relevant_pairs.is_empty() {
        for article in articles.iter().take(10) {
            for market in relevant_markets.iter().take(30) {
                let score = keyword_overlap_score(&article.title, &market.question);
                if score > 0.15 {
                    relevant_pairs.push((article, *market, score));
                }
            }
        }
        relevant_pairs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    }

    emit_status(&app, "match", "Scoring candidate matches...", 0.55);

    relevant_pairs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let mut per_article_count: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    for (article, market, _score) in relevant_pairs.iter().take(40) {
        let count = per_article_count.entry(article.id).or_insert(0);
        if *count >= 3 {
            continue;
        }
        *count += 1;
        let market_price = yes_price(market).unwrap_or(0.5);

        let prompt = format!(
            "Article: '{}'\nMarket: '{}'\nBased on the article, estimate the probability that this market resolves YES. Return ONLY a number between 0.0 and 1.0.",
            article.title, market.question
        );

        let chat_req = CreateChatCompletionRequestArgs::default()
            .model("gpt-4o-mini")
            .messages([
                ChatCompletionRequestMessage::System(async_openai::types::ChatCompletionRequestSystemMessage { content: "You are a quantitative analyst.".to_string(), name: None }),
                ChatCompletionRequestMessage::User(async_openai::types::ChatCompletionRequestUserMessage { content: async_openai::types::ChatCompletionRequestUserMessageContent::Text(prompt), name: None }),
            ])
            .build().map_err(|e| e.to_string())?;

        if let Ok(response) = openai_client.chat().create(chat_req).await {
            if let Some(choice) = response.choices.first() {
                let content = choice.message.content.clone().unwrap_or_default();
                // Strip non-numeric chars just in case
                let clean_content = content.replace(|c: char| !c.is_numeric() && c != '.', "");

                if let Ok(prob) = clean_content.parse::<f64>() {
                    let edge = prob - market_price;
                    let risk = risk_level(market, edge, timeframe_days);

                    let market_slug = market_slug_for_url(market);
                    llm_candidates.push(Recommendation {
                        market_question: market.question.clone(),
                        market_slug,
                        source_article_title: article.title.clone(),
                        source_article_url: article.url.clone().unwrap_or_default(),
                        identified_edge: edge,
                        suggested_outcome: if edge > 0.0 { "YES".to_string() } else { "NO".to_string() },
                        market_price: market_price * 100.0,
                        estimated_prob: prob,
                        reasoning: format!("AI Model ({:.0}%) vs Market ({:.0}%)", prob * 100.0, market_price * 100.0),
                        risk_level: risk,
                        suggested_budget: 0.0,
                        source: article.source.clone(),
                    });
                }
            }
        }
    }

    emit_status(&app, "budget", "Allocating budget by risk tier...", 0.85);

    // 6. Build heuristic candidates so we always return recommendations.
    let mut best_by_market: std::collections::HashMap<String, (&Article, &Market, f64)> =
        std::collections::HashMap::new();
    for (article, market, score) in relevant_pairs.iter() {
        let key = market.id.clone();
        let replace = match best_by_market.get(&key) {
            Some((_, _, best_score)) => score > best_score,
            None => true,
        };
        if replace {
            best_by_market.insert(key, (*article, *market, *score));
        }
    }

    let mut heuristic_candidates: Vec<Recommendation> = Vec::new();
    for (_, (article, market, score)) in best_by_market {
        let market_price = yes_price(market).unwrap_or(0.5);
        let adjustment = (score - 0.5) * 0.2;
        let model_prob = clamp_f64(market_price + adjustment, 0.01, 0.99);
        let edge = model_prob - market_price;
        let risk = risk_level(market, edge, timeframe_days);
        let market_slug = market_slug_for_url(market);
        let liq_score = liquidity_score(market.liquidity.unwrap_or(0.0), market.volume.unwrap_or(0.0));
        let rank_score = edge.abs() * (0.5 + liq_score);

        heuristic_candidates.push(Recommendation {
            market_question: market.question.clone(),
            market_slug,
            source_article_title: article.title.clone(),
            source_article_url: article.url.clone().unwrap_or_default(),
            identified_edge: rank_score,
            suggested_outcome: if edge > 0.0 { "YES".to_string() } else { "NO".to_string() },
            market_price: market_price * 100.0,
            estimated_prob: model_prob,
            reasoning: format!(
                "Heuristic signal (score {:.2}) vs Market ({:.0}%)",
                score,
                market_price * 100.0
            ),
            risk_level: risk,
            suggested_budget: 0.0,
            source: article.source.clone(),
        });
    }

    // Merge and de-duplicate (prefer LLM result when available).
    let mut recommendations: Vec<Recommendation> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    llm_candidates.sort_by(|a, b| {
        b.identified_edge
            .abs()
            .partial_cmp(&a.identified_edge.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for rec in llm_candidates {
        if seen.insert(rec.market_slug.clone()) {
            recommendations.push(rec);
        }
    }

    heuristic_candidates.sort_by(|a, b| {
        b.identified_edge
            .abs()
            .partial_cmp(&a.identified_edge.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for rec in heuristic_candidates {
        if recommendations.len() >= 30 {
            break;
        }
        if seen.insert(rec.market_slug.clone()) {
            recommendations.push(rec);
        }
    }
    let (low_w, med_w, high_w) = budget_weights(timeframe_days);
    let mut low_count = 0;
    let mut med_count = 0;
    let mut high_count = 0;
    for rec in &recommendations {
        match rec.risk_level.as_str() {
            "low-risk" => low_count += 1,
            "medium-risk" => med_count += 1,
            _ => high_count += 1,
        }
    }

    for rec in &mut recommendations {
        let allocation = match rec.risk_level.as_str() {
            "low-risk" => if low_count > 0 { budget * low_w / low_count as f64 } else { 0.0 },
            "medium-risk" => if med_count > 0 { budget * med_w / med_count as f64 } else { 0.0 },
            _ => if high_count > 0 { budget * high_w / high_count as f64 } else { 0.0 },
        };
        rec.suggested_budget = allocation;
    }

    emit_status(&app, "done", "Analysis complete.", 1.0);
    Ok(recommendations)
}
