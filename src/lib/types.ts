export interface Article {
  id: number;
  title: string;
  url?: string;
  score: number;
  descendants: number;
  item_type: string;
}