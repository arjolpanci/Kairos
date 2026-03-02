export interface Article {
  id: number;
  title: string;
  url?: string;
  score: number;
  descendants: number;
  item_type: string;
}

export interface Recommendation {
  market_question: string;
  market_slug: string;
  market_id: string;
  source_article_title: string;
  source_article_url: string;
  identified_edge: number;
  suggested_outcome: "YES" | "NO";
  market_price: number;
  estimated_prob: number;
  reasoning: string;
  risk_level: "low-risk" | "medium-risk" | "high-risk";
  suggested_budget: number;
  source?: string;
  confidence_score: number;
  kelly_fraction: number;
}
