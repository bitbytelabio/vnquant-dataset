use crate::finance::{db::Database, models::Ticker};
use futures::{
    TryStreamExt,
    stream::{self, StreamExt},
};
use std::str::FromStr;

use tradingview::{Country, Interval, history, list_symbols};

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

pub async fn fetch_prices(
    db: Database,
    ticker: &Ticker,
    interval: Interval,
    replay: bool,
) -> anyhow::Result<()> {
    // validate ticker
    if ticker.symbol.is_empty() || ticker.exchange.is_empty() {
        return Err(anyhow::anyhow!("Ticker symbol or exchange is empty"));
    }
    // Check if ticker already exists
    let existing_ticker = db.get_ticker(&ticker.symbol, &ticker.exchange).await?;
    if existing_ticker.is_none() {
        return Err(anyhow::anyhow!(
            "Ticker {} on exchange {} does not exist",
            ticker.symbol,
            ticker.exchange
        ));
    }

    // Fetch historical prices
    let query = history::single::retrieve()
        .symbol(&ticker.symbol)
        .exchange(&ticker.exchange)
        .interval(interval)
        .with_replay(replay);

    let chart_data = query.call().await?;
    // db.update_ticker(&chart_data.symbol_info).await?;
    db.upsert_prices(ticker, interval, &chart_data.data).await?;

    Ok(())
}

pub async fn fetch_prices_batch_stream(
    db: &Database,
    tickers: &[Ticker],
    interval: Interval,
) -> anyhow::Result<()> {
    let data = history::batch::retrieve()
        .symbols(tickers)
        .interval(interval)
        .call()
        .await?;

    // Process chart data as a stream with controlled concurrency
    stream::iter(data.values())
        .map(|chart_data| {
            let db_clone = db.clone();
            let symbol_info = chart_data.symbol_info.clone();
            let data_clone = chart_data.data.clone();

            async move {
                db_clone
                    .upsert_prices(&symbol_info, interval, &data_clone)
                    .await
            }
        })
        .buffer_unordered(10) // Process up to 10 upserts concurrently
        .try_collect::<Vec<_>>() // Collect all results
        .await?;

    Ok(())
}

pub async fn fetch_prices_all_tickers(db: Database, interval: Interval) -> anyhow::Result<()> {
    // Fetch all tickers from the database
    let tickers = db.get_all_tickers().await?;
    if tickers.is_empty() {
        tracing::warn!("No tickers found in the database");
        return Ok(());
    }

    // Fetch prices for all tickers in batches
    fetch_prices_batch_stream(&db, &tickers, interval).await?;

    Ok(())
}

pub async fn fetch_prices_all_tickers_chunked_with_retry(
    db: Database,
    interval: Interval,
    chunk_size: usize,
    max_retries: usize,
) -> anyhow::Result<()> {
    let tickers = db.get_all_tickers().await?;
    if tickers.is_empty() {
        tracing::warn!("No tickers found in the database");
        return Ok(());
    }

    let total_chunks = (tickers.len() + chunk_size - 1) / chunk_size;
    let mut successful_chunks = 0;
    let mut failed_chunks = 0;

    tracing::info!(
        "Processing {} tickers in {} chunks of {}",
        tickers.len(),
        total_chunks,
        chunk_size
    );

    for (chunk_idx, chunk) in tickers.chunks(chunk_size).enumerate() {
        let mut attempts = 0;
        let mut last_error = None;
        if let Some(last_error) = last_error {
            tracing::warn!("Last error: {}", last_error);
        }

        while attempts <= max_retries {
            tracing::info!(
                "Processing chunk {}/{} (attempt {}/{}) with {} tickers",
                chunk_idx + 1,
                total_chunks,
                attempts + 1,
                max_retries + 1,
                chunk.len()
            );

            let start = std::time::Instant::now();

            match fetch_prices_batch_stream(&db, chunk, interval).await {
                Ok(_) => {
                    let duration = start.elapsed();
                    tracing::info!(
                        "Chunk {}/{} completed successfully in {:.2}s",
                        chunk_idx + 1,
                        total_chunks,
                        duration.as_secs_f64()
                    );
                    successful_chunks += 1;
                    break;
                }
                Err(e) => {
                    let duration = start.elapsed();
                    last_error = Some(e);
                    attempts += 1;

                    if attempts <= max_retries {
                        let delay = std::time::Duration::from_secs(2u64.pow(attempts as u32)); // Exponential backoff
                        tracing::warn!(
                            "Chunk {}/{} failed after {:.2}s (attempt {}), retrying in {}s: {}",
                            chunk_idx + 1,
                            total_chunks,
                            duration.as_secs_f64(),
                            attempts,
                            delay.as_secs(),
                            last_error.as_ref().unwrap()
                        );
                        tokio::time::sleep(delay).await;
                    } else {
                        tracing::error!(
                            "Chunk {}/{} failed permanently after {} attempts: {}",
                            chunk_idx + 1,
                            total_chunks,
                            attempts,
                            last_error.as_ref().unwrap()
                        );
                        failed_chunks += 1;
                        break;
                    }
                }
            }
        }

        // Optional: Add delay between chunks
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    tracing::info!(
        "Processing completed: {}/{} chunks successful, {} failed",
        successful_chunks,
        total_chunks,
        failed_chunks
    );

    if failed_chunks > 0 {
        return Err(anyhow::anyhow!(
            "{} chunks failed to process",
            failed_chunks
        ));
    }

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

    #[tokio::test]
    async fn test_fetch_prices() -> anyhow::Result<()> {
        dotenvy::dotenv().ok();
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
        let url = std::env::var("DATABASE_URL")?;
        let db = Database::new(&url).await?;
        let ticker = Ticker::builder()
            .symbol("VCB".to_string())
            .exchange("HOSE".to_string())
            .build();

        fetch_prices(db, &ticker, Interval::OneMinute, true).await?;

        Ok(())
    }
}
