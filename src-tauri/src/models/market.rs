use serde::{Deserialize, Serialize, Deserializer};
use serde_json::Value;

/// Convert a string like "14047.3074" to f64
fn parse_string_to_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse::<f64>().map_err(serde::de::Error::custom)
}

fn parse_string_array<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Input {
        Vec(Vec<String>),
        JsonString(String),
    }

    match Input::deserialize(deserializer)? {
        Input::Vec(v) => Ok(v),
        Input::JsonString(s) => serde_json::from_str(&s).map_err(serde::de::Error::custom),
    }
}

fn parse_string_array_to_f64<'de, D>(deserializer: D) -> Result<Vec<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let arr: Vec<String> = Deserialize::deserialize(deserializer)?;
    arr.into_iter()
        .map(|s| s.parse::<f64>().map_err(serde::de::Error::custom))
        .collect()
}

fn parse_f64_array<'de, D>(deserializer: D) -> Result<Vec<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Input {
        VecF64(Vec<f64>),
        VecString(Vec<String>),
        JsonString(String),
    }

    let values: Vec<Value> = match Input::deserialize(deserializer)? {
        Input::VecF64(v) => return Ok(v),
        Input::VecString(v) => {
            return v
                .into_iter()
                .map(|s| s.parse::<f64>().map_err(serde::de::Error::custom))
                .collect()
        }
        Input::JsonString(s) => serde_json::from_str(&s).map_err(serde::de::Error::custom)?,
    };

    values
        .into_iter()
        .map(|v| match v {
            Value::Number(n) => n.as_f64().ok_or_else(|| serde::de::Error::custom("invalid number")),
            Value::String(s) => s.parse::<f64>().map_err(serde::de::Error::custom),
            _ => Err(serde::de::Error::custom("invalid number value")),
        })
        .collect()
}

fn parse_optional_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(Value::Number(n)) => Ok(n.as_f64()),
        Some(Value::String(s)) => s.parse::<f64>().map(Some).map_err(serde::de::Error::custom),
        Some(Value::Null) => Ok(None),
        _ => Err(serde::de::Error::custom("invalid number value")),
    }
}

/// Represents a Market from Polymarket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub id: String,
    pub question: String,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default, deserialize_with = "parse_string_array")]
    pub outcomes: Vec<String>,
    #[serde(default, rename = "outcomePrices", deserialize_with = "parse_f64_array")]
    pub outcome_prices: Vec<f64>,
    #[serde(default, deserialize_with = "parse_optional_f64")]
    pub volume: Option<f64>,
    #[serde(default, deserialize_with = "parse_optional_f64")]
    pub liquidity: Option<f64>,
    #[serde(default, rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(default, rename = "endDate")]
    pub end_date: Option<String>,
    #[serde(default)]
    pub active: Option<bool>,
    #[serde(default)]
    pub closed: Option<bool>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub events: Vec<Event>,
    #[serde(default)]
    pub series: Vec<Series>,
}


/// Represents an Event inside a Market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub ticker: String,
    pub slug: String,
    pub title: String,
    pub description: Option<String>,
    #[serde(default, rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(default, rename = "endDate")]
    pub end_date: Option<String>,
    #[serde(default)]
    pub series: Vec<Series>,
}

/// Represents a Series inside an Event or Market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Series {
    pub id: String,
    pub ticker: String,
    pub slug: String,
    pub title: String,
    #[serde(rename = "seriesType")]
    pub series_type: Option<String>,
    pub recurrence: Option<String>,
    pub active: bool,
    pub closed: bool,
    pub archived: bool,
}
