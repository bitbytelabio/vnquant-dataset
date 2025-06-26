use crate::finance::{db::Database, models::Ticker};
use std::str::FromStr;
use tradingview::{Country, list_symbols};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExchangeConfig {
    pub exchange: String,
    pub country: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TVConfigMap {
    pub exchanges: Vec<ExchangeConfig>,
}

pub async fn fetch_tickers(db: Database) -> anyhow::Result<()> {
    let exchanges_str = include_str!("exchanges.json");
    let config: TVConfigMap = serde_json::from_str(exchanges_str)?;
    let mut tickers = Vec::new();

    for exchange_config in config.exchanges {
        let country_opt = if let Some(country) = exchange_config.country.clone() {
            Country::from_str(&country).ok()
        } else {
            None
        };

        let query = list_symbols()
            .exchange(&exchange_config.exchange)
            .maybe_country(country_opt);

        let symbols = query.call().await?;
        tracing::info!(
            "Fetched {} symbols from exchange: {} (country: {})",
            symbols.len(),
            exchange_config.exchange,
            exchange_config.country.as_deref().unwrap_or("N/A")
        );

        tickers.extend(symbols.into_iter().map(|s| Ticker::from(s)));
    }

    db.upsert_tickers(&tickers).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finance::db::Database;

    #[tokio::test]
    async fn test_fetch_tickers() -> anyhow::Result<()> {
        dotenvy::dotenv().ok();
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
        let url = std::env::var("DATABASE_URL")?;
        let db = Database::new(&url).await?;
        fetch_tickers(db).await?;
        Ok(())
    }
}
