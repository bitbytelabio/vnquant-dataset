use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Default)]
pub struct Ticker {
    pub symbol: String,
    pub exchange: String,
    pub description: Option<String>,
    pub currency: Option<String>,
    pub country: Option<String>,
    pub market_type: Option<String>,
    pub industry: Option<String>,
    pub sector: Option<String>,
    pub founded: Option<i64>,
}

impl From<tradingview::Symbol> for Ticker {
    fn from(symbol: tradingview::Symbol) -> Self {
        Self {
            symbol: symbol.symbol,
            exchange: symbol.exchange,
            description: Some(symbol.description),
            currency: Some(symbol.currency_code),
            country: Some(symbol.country_code),
            market_type: Some(symbol.market_type),
            industry: None,
            sector: None,
            founded: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Default)]
pub struct Candle {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Default)]
pub struct Indicator {
    pub timestamp: DateTime<Utc>,
    pub indicator_type: String,
    pub value: Option<f64>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlFeatures {
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
