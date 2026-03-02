use crate::api::{news::fetch_all_articles, polymarket::fetch_active_markets};
use crate::models::{article::Article, market::Market};
use crate::storage::cache::Cache;
use async_openai::{
    types::{
        CreateChatCompletionRequestArgs, ChatCompletionRequestMessage,
        CreateEmbeddingRequestArgs,
    },
    Client as OpenAIClient,
};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Recommendation {
    pub market_question: String,
    pub market_slug: String,
    pub market_id: String,
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
    pub confidence_score: f64,
    pub kelly_fraction: f64,
}

#[derive(Debug, Serialize, Clone)]
struct AnalysisStatus {
    step: String,
    message: String,
    progress: f32,
}

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

fn ambiguity_penalty(question: &str) -> f64 {
    let q = question.to_lowercase();
    let mut penalty: f64 = 0.0;
    for term in [
        "likely",
        "probably",
        "expected",
        "may",
        "might",
        "could",
        "as of",
        "according to",
        "estimate",
        "rumor",
        "reported",
    ] {
        if q.contains(term) {
            penalty += 0.05;
        }
    }
    penalty.min(0.3)
}

fn market_quality_score(market: &Market) -> f64 {
    let liq = liquidity_score(market.liquidity.unwrap_or(0.0), market.volume.unwrap_or(0.0));
    let ambiguity = ambiguity_penalty(&market.question);
    (liq * (1.0 - ambiguity)).clamp(0.0, 1.0)
}

fn source_weight(article: &Article) -> f64 {
    let source = article.source.as_deref().unwrap_or("").to_lowercase();
    let mut weight = 1.0;
    for (pattern, w) in [
        ("reuters", 1.2),
        ("ap", 1.1),
        ("bloomberg", 1.2),
        ("financial times", 1.2),
        ("wsj", 1.1),
        ("bbc", 1.1),
        ("npr", 1.1),
        ("cnbc", 1.05),
        ("guardian", 1.05),
    ] {
        if source.contains(pattern) {
            weight = w;
            break;
        }
    }
    weight
}

fn recency_weight(published_at: &Option<String>) -> f64 {
    let Some(raw) = published_at else { return 0.7; };
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        let age_hours = (Utc::now() - dt.with_timezone(&Utc)).num_hours().max(0) as f64;
        let half_life = 48.0;
        return (0.5_f64).powf(age_hours / half_life).max(0.2);
    }
    0.7
}

// Helper functions for LLM response parsing
fn extract_probability(content: &str) -> Option<f64> {
    for line in content.lines() {
        if line.to_lowercase().starts_with("probability:") {
            let num_str = line.split(':').nth(1).unwrap_or("").trim();
            return num_str.parse::<f64>().ok();
        }
    }
    // Fallback: find any number between 0 and 1
    let re = regex::Regex::new(r"\b(0\.\d+|1\.0|0|1)\b").ok()?;
    re.find(content).and_then(|m| m.as_str().parse::<f64>().ok())
}

fn extract_confidence(content: &str) -> String {
    for line in content.lines() {
        let lower = line.to_lowercase();
        if lower.starts_with("confidence:") {
            if lower.contains("high") {
                return "high".to_string();
            } else if lower.contains("medium") {
                return "medium".to_string();
            } else if lower.contains("low") {
                return "low".to_string();
            }
        }
    }
    "medium".to_string()
}

fn extract_edge_analysis(content: &str) -> String {
    for line in content.lines() {
        let lower = line.to_lowercase();
        if lower.starts_with("edge_analysis:") || lower.starts_with("edge analysis:") {
            return line.split(':').nth(1).unwrap_or("").trim().to_string();
        }
    }
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.ends_with(':') {
            return trimmed.to_string();
        }
    }
    "AI analysis completed".to_string()
}

// Kelly Criterion calculation
fn calculate_kelly_fraction(prob: f64, market_price: f64) -> f64 {
    if prob <= 0.0 || prob >= 1.0 || market_price <= 0.0 || market_price >= 1.0 {
        return 0.0;
    }

    let edge = prob - market_price;
    let variance = market_price * (1.0 - market_price);

    let kelly = if variance > 0.0 {
        (edge / variance) * 0.5
    } else {
        0.0
    };

    kelly.clamp(0.0, 0.25)
}

fn allocate_budget_with_kelly(
    mut recommendations: Vec<Recommendation>,
    budget: f64,
    timeframe_days: i64,
) -> Vec<Recommendation> {
    recommendations.retain(|r| r.identified_edge.abs() > 0.02);

    if recommendations.is_empty() {
        return recommendations;
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

    let low_budget = budget * low_w;
    let med_budget = budget * med_w;
    let high_budget = budget * high_w;

    let mut low_kelly_sum = 0.0;
    let mut med_kelly_sum = 0.0;
    let mut high_kelly_sum = 0.0;

    for rec in &recommendations {
        match rec.risk_level.as_str() {
            "low-risk" => low_kelly_sum += rec.kelly_fraction,
            "medium-risk" => med_kelly_sum += rec.kelly_fraction,
            _ => high_kelly_sum += rec.kelly_fraction,
        }
    }

    for rec in &mut recommendations {
        let tier_budget = match rec.risk_level.as_str() {
            "low-risk" => low_budget,
            "medium-risk" => med_budget,
            _ => high_budget,
        };

        let tier_kelly_sum = match rec.risk_level.as_str() {
            "low-risk" => low_kelly_sum,
            "medium-risk" => med_kelly_sum,
            _ => high_kelly_sum,
        };

        if tier_kelly_sum > 0.0 {
            let normalized_kelly = rec.kelly_fraction / tier_kelly_sum;
            rec.suggested_budget = (tier_budget * normalized_kelly).max(1.0).min(budget * 0.25);
        } else {
            let count = match rec.risk_level.as_str() {
                "low-risk" => low_count.max(1),
                "medium-risk" => med_count.max(1),
                _ => high_count.max(1),
            };
            rec.suggested_budget = (tier_budget / count as f64).max(1.0).min(budget * 0.25);
        }
    }

    recommendations
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

    emit_status(&app, "fetch", "Fetching articles and markets...", 0.1);
    let mut cache = Cache::new(&app).ok();
    let cached_articles = cache
        .as_ref()
        .and_then(|c| c.get_recent_articles(48).ok())
        .unwrap_or_default();
    let cached_markets = cache
        .as_ref()
        .and_then(|c| c.get_recent_markets(24).ok())
        .unwrap_or_default();

    let (articles_res, markets_res) = tokio::join!(fetch_all_articles(true), fetch_active_markets());
    let mut articles = articles_res.map_err(|e| e.to_string())?;
    let mut markets = markets_res.map_err(|e| e.to_string())?;

    if !cached_articles.is_empty() {
        articles.extend(cached_articles);
    }
    if !cached_markets.is_empty() {
        markets.extend(cached_markets);
    }

    let mut seen_articles: std::collections::HashSet<String> = std::collections::HashSet::new();
    articles.retain(|article| {
        let key = article
            .url
            .as_ref()
            .cloned()
            .unwrap_or_else(|| article.title.clone());
        seen_articles.insert(key)
    });

    let mut seen_markets: std::collections::HashSet<String> = std::collections::HashSet::new();
    markets.retain(|market| seen_markets.insert(market.id.clone()));

    if let Some(cache) = cache.as_mut() {
        let _ = cache.upsert_articles(&articles);
        let _ = cache.upsert_markets(&markets);
    }

    if articles.is_empty() || markets.is_empty() {
        return Ok(vec![]);
    }

    emit_status(
        &app,
        "filter",
        &format!("Processing {} articles and {} markets...", articles.len(), markets.len()),
        0.2,
    );

    let markets_by_timeframe: Vec<&Market> = markets.iter().collect();

    let article_texts: Vec<String> = articles
        .iter()
        .take(150)
        .map(|a| {
            let mut text = a.title.clone();
            if let Some(summary) = &a.summary {
                text.push_str(" | ");
                text.push_str(summary);
            }
            text
        })
        .collect();

    let mut keywords: Vec<String> = crate::api::news::extract_keywords_from_titles(&article_texts, 25);
    for kw in ["election", "inflation", "fed", "rates", "crypto", "earnings", "sports", "politics", "ai", "oil", "trump", "biden", "war", "ukraine", "israel", "gaza", "trade", "tariff", "gdp", "recession", "stock", "market", "bitcoin", "ethereum", "nft"] {
        keywords.push(kw.to_string());
    }
    keywords.sort();
    keywords.dedup();

    emit_status(
        &app,
        "filter",
        &format!("Filtering {} markets by keywords...", markets_by_timeframe.len()),
        0.25,
    );

    let mut relevant_markets: Vec<&Market> = markets_by_timeframe
        .iter()
        .copied()
        .filter(|market| {
            if market_quality_score(market) < 0.15 {
                return false;
            }
            let question = normalize_text(&market.question);
            keywords.iter().any(|kw| question.contains(kw))
        })
        .collect();
    if relevant_markets.is_empty() {
        let mut sorted_markets: Vec<&Market> = markets_by_timeframe.iter().copied().collect();
        sorted_markets.sort_by(|a, b| {
            let score_a = market_quality_score(a);
            let score_b = market_quality_score(b);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        relevant_markets = sorted_markets.into_iter().take(500).collect();
    }

    emit_status(
        &app,
        "embedding",
        &format!("Generating embeddings for {} articles and {} markets...", article_texts.len(), relevant_markets.len()),
        0.35,
    );

    let market_questions: Vec<String> = relevant_markets
        .iter()
        .take(1000)
        .map(|m| market_text(m))
        .collect();

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

    let mut market_embedding_map: std::collections::HashMap<String, Vec<f32>> = std::collections::HashMap::new();
    for (idx, market) in relevant_markets.iter().take(1000).enumerate() {
        if let Some(emb) = market_embeddings.data.get(idx) {
            market_embedding_map.insert(market.id.clone(), emb.embedding.clone());
        }
    }

    emit_status(&app, "match", "Finding article-market matches...", 0.45);

    let mut relevant_pairs: Vec<(&Article, &Market, f64)> = Vec::new();

    for (i, art_emb) in article_embeddings.data.iter().enumerate() {
        let mut scored: Vec<(&Market, f64)> = Vec::new();
        for (j, mar_emb) in market_embeddings.data.iter().enumerate() {
            let cosine = cosine_similarity(&art_emb.embedding, &mar_emb.embedding);
            if let Some(market) = relevant_markets.get(j) {
                let article_text = article_texts.get(i).unwrap_or(&articles[i].title);
                let overlap = keyword_overlap_score(article_text, &market.question);
                let score = 0.7 * cosine + 0.3 * overlap;
                if score > 0.22 {
                    scored.push((*market, score));
                }
            }
        }
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (market, score) in scored.into_iter().take(15) {
            relevant_pairs.push((&articles[i], market, score));
        }
    }

    emit_status(&app, "llm", "Running deep analysis on top matches...", 0.55);

    let mut llm_candidates: Vec<Recommendation> = Vec::new();

    relevant_pairs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let mut seen_markets_for_llm: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut llm_analysis_candidates: Vec<(&Article, &Market, f64)> = Vec::new();

    for (article, market, score) in relevant_pairs.iter() {
        if seen_markets_for_llm.insert(market.id.clone()) && *score > 0.3 {
            llm_analysis_candidates.push((*article, *market, *score));
        }
        if llm_analysis_candidates.len() >= 25 {
            break;
        }
    }

    for (i, (article, market, match_score)) in llm_analysis_candidates.iter().enumerate() {
        emit_status(&app, "llm", &format!("Analyzing match {}/{}...", i + 1, llm_analysis_candidates.len()), 0.55 + (i as f32 * 0.015));

        let market_price = yes_price(market).unwrap_or(0.5);

        let article_context = if let Some(summary) = &article.summary {
            format!("{}\n\nSummary: {}", article.title, summary)
        } else {
            article.title.clone()
        };

        let prompt = format!(
            r#"You are an expert prediction market analyst. Analyze the relationship between this news article and prediction market.

ARTICLE:
{}

MARKET QUESTION: "{}"
MARKET OUTCOMES: {:?}
CURRENT MARKET PRICE: {:.1}¢ (implies {:.0}% probability)

Analyze and respond in this exact format:

PROBABILITY: [number between 0.0 and 1.0 - your calibrated probability estimate]
CONFIDENCE: [high/medium/low - how confident you are in this estimate]
KEY_CLAIMS: [2-3 key claims from the article relevant to this market]
EDGE_ANALYSIS: [1-2 sentences on why the market price might be wrong]
RISKS: [1-2 sentences on what could make your estimate wrong]"#,
            article_context,
            market.question,
            market.outcomes,
            market_price * 100.0,
            market_price * 100.0
        );

        let chat_req = CreateChatCompletionRequestArgs::default()
            .model("gpt-4o-mini")
            .messages([
                ChatCompletionRequestMessage::System(async_openai::types::ChatCompletionRequestSystemMessage {
                    content: "You are an expert prediction market analyst with deep expertise in extracting signal from news and calibrating probability estimates.".to_string(),
                    name: None
                }),
                ChatCompletionRequestMessage::User(async_openai::types::ChatCompletionRequestUserMessage {
                    content: async_openai::types::ChatCompletionRequestUserMessageContent::Text(prompt),
                    name: None
                }),
            ])
            .temperature(0.1)
            .max_tokens(500_u32)
            .build().map_err(|e| e.to_string())?;

        if let Ok(response) = openai_client.chat().create(chat_req).await {
            if let Some(choice) = response.choices.first() {
                let content = choice.message.content.clone().unwrap_or_default();

                let prob = extract_probability(&content).unwrap_or(0.5);
                let confidence = extract_confidence(&content);
                let edge_analysis = extract_edge_analysis(&content);

                let edge = prob - market_price;
                let risk = risk_level(market, edge, timeframe_days);
                let quality = market_quality_score(market);
                let signal_weight = source_weight(article) * recency_weight(&article.published_at);
                let confidence_multiplier = match confidence.as_str() {
                    "high" => 1.0,
                    "medium" => 0.7,
                    _ => 0.4,
                };
                let adjusted_edge = edge * quality * signal_weight * confidence_multiplier;

                let market_slug = market_slug_for_url(market);

                let kelly = calculate_kelly_fraction(prob, market_price);

                llm_candidates.push(Recommendation {
                    market_question: market.question.clone(),
                    market_slug,
                    market_id: market.id.clone(),
                    source_article_title: article.title.clone(),
                    source_article_url: article.url.clone().unwrap_or_default(),
                    identified_edge: adjusted_edge,
                    suggested_outcome: if edge > 0.0 { "YES".to_string() } else { "NO".to_string() },
                    market_price: market_price * 100.0,
                    estimated_prob: prob,
                    reasoning: format!(
                        "AI: {:.0}% vs Market: {:.0}% | {} | Quality: {:.2}",
                        prob * 100.0,
                        market_price * 100.0,
                        edge_analysis,
                        quality
                    ),
                    risk_level: risk,
                    suggested_budget: 0.0,
                    source: article.source.clone(),
                    confidence_score: confidence_multiplier,
                    kelly_fraction: kelly,
                });
            }
        }
    }

    emit_status(&app, "budget", "Allocating budget by risk tier...", 0.85);

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
        let quality = market_quality_score(market);
        let signal_weight = source_weight(article) * recency_weight(&article.published_at);
        let rank_score = edge.abs() * (0.5 + liq_score) * quality * signal_weight;

        heuristic_candidates.push(Recommendation {
            market_question: market.question.clone(),
            market_slug,
            market_id: market.id.clone(),
            source_article_title: article.title.clone(),
            source_article_url: article.url.clone().unwrap_or_default(),
            identified_edge: rank_score,
            suggested_outcome: if edge > 0.0 { "YES".to_string() } else { "NO".to_string() },
            market_price: market_price * 100.0,
            estimated_prob: model_prob,
            reasoning: format!(
                "Heuristic signal (score {:.2}) vs Market ({:.0}%), quality {:.2}, signal {:.2}",
                score,
                market_price * 100.0,
                quality,
                signal_weight
            ),
            risk_level: risk,
            suggested_budget: 0.0,
            source: article.source.clone(),
            confidence_score: 0.5,
            kelly_fraction: 0.0,
        });
    }

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
        if recommendations.len() >= 50 {
            break;
        }
        if seen.insert(rec.market_slug.clone()) {
            recommendations.push(rec);
        }
    }

    recommendations.sort_by(|a, b| {
        b.identified_edge
            .abs()
            .partial_cmp(&a.identified_edge.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut clustered: Vec<Recommendation> = Vec::new();
    let mut clusters: Vec<Vec<f32>> = Vec::new();
    let mut cluster_counts: Vec<usize> = Vec::new();
    let max_per_cluster = 2;

    for rec in recommendations.into_iter() {
        let emb = market_embedding_map.get(&rec.market_id);
        if let Some(embedding) = emb {
            let mut target: Option<usize> = None;
            for (idx, rep) in clusters.iter().enumerate() {
                if cosine_similarity(embedding, rep) > 0.86 {
                    target = Some(idx);
                    break;
                }
            }
            if let Some(idx) = target {
                if cluster_counts[idx] < max_per_cluster {
                    cluster_counts[idx] += 1;
                    clustered.push(rec);
                }
            } else {
                clusters.push(embedding.clone());
                cluster_counts.push(1);
                clustered.push(rec);
            }
        } else {
            clustered.push(rec);
        }
        if clustered.len() >= 50 {
            break;
        }
    }

    let mut recommendations = clustered;
    recommendations = allocate_budget_with_kelly(recommendations, budget, timeframe_days);

    emit_status(&app, "done", "Analysis complete.", 1.0);
    Ok(recommendations)
}
