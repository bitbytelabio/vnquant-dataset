use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tradingview::Interval;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Ticker {
    pub symbol: String,
    pub exchange: String,
    pub description: Option<String>,
    pub currency: Option<String>,
    pub country: Option<String>,
    pub market_type: Option<String>,
    pub industry: Option<String>,
    pub sector: Option<String>,
    pub founded: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Candle {
    pub symbol: String,
    pub exchange: String,
    pub interval: Interval,
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Indicator {
    pub symbol: String,
    pub exchange: String,
    pub interval: Interval,
    pub timestamp: DateTime<Utc>,
    pub indicator_type: String,
    pub value: Option<f64>,
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlFeatures {
    pub symbol: String,
    pub exchange: String,
    pub interval: Interval,
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub rsi: Option<f64>,
    pub mfi: Option<f64>,
    pub sma_20: Option<f64>,
    pub ema_12: Option<f64>,
    pub price_change_pct: Option<f64>,
    pub volatility_pct: Option<f64>,
}
