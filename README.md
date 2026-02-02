# Kairos

**AI-Driven Prediction Market Research App**

Kairos is a desktop application designed to surface potential mispricings in prediction markets (like Polymarket) by correlating news signals with active markets. It suggests allocations; it does not place trades.

## Project Overview

The goal is a local, privacy-focused tool that assists users in making data-backed decisions. The app automates the following workflow:

1.  **Data Aggregation:** Fetch articles from multiple sources (Hacker News, NewsAPI, and RSS feeds) plus active Polymarket markets.
2.  **Semantic Analysis:** Use OpenAI embeddings to compare news vs. market text; optionally use LLM calls to estimate implied probabilities.
3.  **Market Matching:** Connect news to relevant markets via vector similarity + keyword overlap.
4.  **Strategy Recommendation:** Rank markets by estimated edge, liquidity, and risk; split suggested budget across low/medium/high risk.

## Tech Stack

**Backend (Rust)**
*   **Tauri v2:** Application framework.
*   **Tokio:** Asynchronous runtime for concurrent fetching.
*   **Reqwest:** HTTP client for API interaction.
*   **Async-OpenAI:** Embeddings + LLM scoring.
*   **Feed-rs:** RSS ingestion.

**Frontend (Svelte)**
*   **Svelte:** UI Framework.
*   **TailwindCSS:** Styling and design system.
*   **Vite:** Build tool.

## Getting Started

### Prerequisites

*   Rust (latest stable)
*   Node.js (LTS)
*   pnpm (recommended)
*   OpenAI API key (for embeddings + LLM scoring)
*   NewsAPI key (optional, for additional news coverage)

### Installation

1.  Clone the repository:
    ```bash
    git clone https://github.com/yourusername/kairos.git
    cd kairos
    ```

2.  Install frontend dependencies:
    ```bash
    pnpm install
    ```

3.  Configure environment variables (optional but recommended):
    ```bash
    # src-tauri/.env
    OPENAI_API_KEY=your_key_here
    NEWSAPI_KEY=your_key_here
    ```

4.  Run the application in development mode:
    ```bash
    pnpm tauri dev
    ```

## Current Status

This project is in active development. Core data ingestion and analysis are working, but tuning, ranking, and UI polish are ongoing.

### Implemented
*   News aggregation (Hacker News, NewsAPI, RSS feeds).
*   Polymarket scanning (Gamma API, multiple pages/sorts).
*   Embedding-based market matching + heuristic fallback.
*   Risk-tiered recommendations + budget allocation.
*   Progress updates streamed to the UI during analysis.

### In Progress
*   Better market quality filters (liquidity, resolution clarity).
*   Improved topic clustering and correlation handling.
*   More external signal integrations.

## License

Distributed under the MIT License.
