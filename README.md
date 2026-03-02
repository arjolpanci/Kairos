# Kairos

**AI-Driven Prediction Market Intelligence Platform**

Kairos is a desktop application that surfaces potential mispricings in prediction markets (primarily Polymarket) by analyzing news signals and correlating them with active markets. It uses advanced NLP, semantic embeddings, and Kelly criterion-based position sizing to generate actionable betting recommendations.

> **Disclaimer:** Kairos provides informational recommendations only. It does not place trades. Always verify sources and do your own research before allocating capital.

## How It Works

1. **Data Aggregation:** Fetches articles from 40+ sources including Hacker News, NewsAPI, GDELT, and major news RSS feeds (Reuters, Bloomberg, BBC, NYT, etc.)
2. **Market Discovery:** Retrieves active Polymarket markets with quality filtering (liquidity, volume thresholds)
3. **Semantic Matching:** Uses OpenAI embeddings to match articles with relevant markets via cosine similarity
4. **AI Analysis:** GPT-4o-mini analyzes article-market pairs to estimate probabilities and identify edge
5. **Risk Assessment:** Classifies recommendations into low/medium/high risk tiers based on liquidity, time horizon, and edge magnitude
6. **Position Sizing:** Applies Kelly criterion for optimal bet sizing within each risk tier

## Tech Stack

**Backend (Rust + Tauri v2)**
- **Tauri v2:** Cross-platform desktop framework
- **Tokio:** Async runtime for concurrent API fetching
- **Reqwest:** HTTP client for news and market APIs
- **Async-OpenAI:** Embeddings and LLM analysis
- **Feed-rs:** RSS/Atom feed parsing
- **Rusqlite:** Local caching of articles and markets
- **Regex:** LLM response parsing

**Frontend (Svelte + TypeScript)**
- **Svelte 5:** Reactive UI framework
- **TailwindCSS:** Utility-first styling
- **Vite:** Build tool and dev server

## Getting Started

### Prerequisites

- Rust (latest stable)
- Node.js (LTS)
- pnpm (recommended)
- OpenAI API key (required for embeddings + LLM analysis)
- NewsAPI key (optional, for additional news coverage)

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/kairos.git
   cd kairos
   ```

2. Install frontend dependencies:
   ```bash
   pnpm install
   ```

3. Configure environment variables:
   ```bash
   # src-tauri/.env
   OPENAI_API_KEY=your_key_here
   NEWSAPI_KEY=your_key_here
   ```

4. Run in development mode:
   ```bash
   pnpm tauri dev
   ```

## Features

### Implemented

- **Multi-source news aggregation** (40+ RSS feeds, Hacker News, NewsAPI, GDELT)
- **Polymarket integration** with quality filtering
- **Semantic embeddings** for article-market matching (text-embedding-3-small)
- **LLM-powered analysis** with structured probability estimates
- **Risk-tiered recommendations** (low/medium/high)
- **Kelly criterion position sizing** with fractional Kelly for risk management
- **Topic clustering** to reduce correlated exposure
- **Source weighting** (Reuters, Bloomberg, AP get higher weights)
- **Recency decay** for article relevance
- **Progress streaming** to UI during analysis
- **Local SQLite caching** for articles and markets

### Analysis Pipeline

1. **Fetch Phase:** Retrieves up to 200 HN stories, 500 NewsAPI articles, and 40+ RSS feeds
2. **Filter Phase:** Deduplicates and filters articles; filters markets by liquidity (> $5k) and volume (> $10k)
3. **Embedding Phase:** Generates embeddings for top 150 articles and up to 1000 markets
4. **Matching Phase:** Cosine similarity + keyword overlap scoring
5. **LLM Analysis Phase:** Deep analysis of top 25 article-market matches
6. **Clustering Phase:** Groups similar markets to limit correlated bets
7. **Allocation Phase:** Kelly-based budget allocation across risk tiers

## Configuration

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `OPENAI_API_KEY` | Yes | For embeddings and LLM analysis |
| `NEWSAPI_KEY` | No | For additional news coverage |
| `ECON_CALENDAR_URL` | No | External economic calendar RSS |
| `POLL_FEED_URL` | No | External polling data feed |

### Risk Tiers

| Tier | Budget Allocation | Characteristics |
|------|-------------------|-----------------|
| Low Risk | 50-70% | High liquidity, near-term resolution, modest edge |
| Medium Risk | 20-30% | Moderate liquidity, medium-term, solid edge |
| High Risk | 10-20% | Lower liquidity, long-term, high edge |

## Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   News Sources  │────▶│  Rust Backend   │────▶│  OpenAI APIs    │
│  (HN/API/RSS)   │     │  (Tauri/Core)   │     │ (Embeddings+LLM)│
└─────────────────┘     └─────────────────┘     └─────────────────┘
                               │
                               ▼
                        ┌─────────────────┐
                        │  SQLite Cache   │
                        └─────────────────┘
                               │
                               ▼
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Svelte Frontend│◀────│  Analysis Engine│◀────│ Polymarket API  │
│   (UI/Results)  │     │ (Matching/Risk) │     │  (Markets)      │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

## Current Limitations

- Article content is limited to titles and summaries (full text extraction not implemented)
- No historical backtesting or performance tracking yet
- Polymarket API doesn't provide historical price data for momentum analysis
- LLM analysis limited to 25 top matches to manage API costs
- No automated trade execution (manual only)

## Future Roadmap

- [ ] Full article text extraction for deeper analysis
- [ ] Historical performance tracking and calibration
- [ ] Additional prediction markets (Kalshi, etc.)
- [ ] Momentum and technical indicators
- [ ] Custom user-defined watchlists
- [ ] Export recommendations to CSV
- [ ] Confidence calibration based on historical accuracy

## License

Distributed under the MIT License.
