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

#[bon::bon]
impl Ticker {
    #[builder]
    pub fn new(
        symbol: String,
        exchange: String,
        description: Option<String>,
        currency: Option<String>,
        country: Option<String>,
        market_type: Option<String>,
        industry: Option<String>,
        sector: Option<String>,
        founded: Option<i64>,
    ) -> Self {
        Self {
            symbol,
            exchange,
            description,
            currency,
            country,
            market_type,
            industry,
            sector,
            founded,
        }
    }
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

impl tradingview::MarketSymbol for Ticker {
    fn new<S: Into<String>>(symbol: S, exchange: S) -> Self {
        Self {
            symbol: symbol.into(),
            exchange: exchange.into(),
            description: None,
            currency: None,
            country: None,
            market_type: None,
            industry: None,
            sector: None,
            founded: None,
        }
    }

    fn symbol(&self) -> &str {
        &self.symbol
    }

    fn exchange(&self) -> &str {
        &self.exchange
    }

    fn currency(&self) -> &str {
        self.currency.as_deref().unwrap_or("N/A")
    }

    fn market_type(&self) -> tradingview::MarketType {
        match self.market_type.as_deref() {
            Some("stock") => tradingview::MarketType::Stocks(tradingview::StocksType::All),
            Some("forex") => tradingview::MarketType::Forex,
            Some("crypto") => tradingview::MarketType::Crypto(tradingview::CryptoType::All),
            Some("futures") => tradingview::MarketType::Futures,
            _ => tradingview::MarketType::All,
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

impl tradingview::OHLCV for Candle {
    fn timestamp(&self) -> i64 {
        self.timestamp.timestamp_millis()
    }

    fn open(&self) -> f64 {
        self.open
    }

    fn high(&self) -> f64 {
        self.high
    }

    fn low(&self) -> f64 {
        self.low
    }

    fn close(&self) -> f64 {
        self.close
    }

    fn volume(&self) -> f64 {
        self.volume
    }

    fn datetime(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn is_ohlcv(&self) -> bool {
        true
    }
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
