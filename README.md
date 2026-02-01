# Kairos

**AI-Driven Prediction Market Engine**

Kairos is a desktop application designed to identify investment opportunities in prediction markets (like Polymarket) by correlating real-time news events with active betting contracts.

## Project Overview

The goal of this project is to build a local, privacy-focused tool that assists users in making data-backed decisions. The application will automate the following workflow:

1.  **Data Aggregation:** Fetch relevant articles from high-signal sources (e.g., HackerNews, Financial feeds) and active markets from prediction platforms.
2.  **Semantic Analysis:** Use LLMs to determine the sentiment and implied probability of news events.
3.  **Market Matching:** Connect vague news headlines to specific betting contracts using vector similarity.
4.  **Strategy Recommendation:** Provide calculated investment suggestions based on user-defined budgets and timeframes.

## Tech Stack

**Backend (Rust)**
*   **Tauri v2:** Application framework.
*   **Tokio:** Asynchronous runtime for concurrent fetching.
*   **Reqwest:** HTTP client for API interaction.
*   **FastEmbed:** Local vector embedding generation.

**Frontend (Svelte)**
*   **Svelte:** UI Framework.
*   **TailwindCSS:** Styling and design system.
*   **Vite:** Build tool.

## Getting Started

### Prerequisites

*   Rust (latest stable)
*   Node.js (LTS)
*   pnpm (recommended)

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

3.  Run the application in development mode:
    ```bash
    pnpm tauri dev
    ```

## Roadmap

This project is currently in the initial setup phase.

*   [X] Project scaffolding (Rust + Svelte).
*   [X] Basic UI layout with Tailwind.
*   [ ] Integration of News APIs.
*   [ ] Integration of Prediction Market APIs.
*   [ ] Implementation of Analysis Engine.

## License

Distributed under the MIT License.