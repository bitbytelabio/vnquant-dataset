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

pub async fn fetch_tickers(db: Database, path: &str) -> anyhow::Result<()> {
    let exchanges_str = std::fs::read_to_string(path)?;

    let config: TVConfigMap = serde_json::from_str(&exchanges_str)?;
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

        tickers.extend(symbols.into_iter().map(Ticker::from));
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
        db.upsert_tickers(&[ticker.clone()]).await?;
        tracing::info!(
            "Inserted new ticker: {} on exchange: {}",
            ticker.symbol,
            ticker.exchange
        );
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

pub async fn fetch_prices_batch(
    db: &Database,
    tickers: &[Ticker],
    interval: Interval,
) -> anyhow::Result<()> {
    // Validate tickers
    if tickers.is_empty() {
        return Err(anyhow::anyhow!("No tickers provided for batch processing"));
    }
    for ticker in tickers {
        if ticker.symbol.is_empty() || ticker.exchange.is_empty() {
            return Err(anyhow::anyhow!(
                "Ticker symbol or exchange is empty for ticker: {:?}",
                ticker
            ));
        }
    }

    db.upsert_tickers(tickers).await?;

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
                db_clone.upsert_ticker(&symbol_info).await?;
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

pub async fn fetch_prices_all(
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

    let total_chunks = tickers.len().div_ceil(chunk_size);
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

            match fetch_prices_batch(&db, chunk, interval).await {
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

pub async fn fetch_intraday_prices(
    db: &Database,
    tickers: &[Ticker],
    interval: Interval,
    concurrency: usize,
    replay: bool,
    update_existing: bool,
) -> anyhow::Result<()> {
    if update_existing {
        // Update existing tickers in the database
        db.upsert_tickers(tickers).await?;
    }

    let total_tickers = tickers.len();
    let progress_interval = std::cmp::max(total_tickers / 20, 1); // Report progress every 5%

    tracing::info!(
        "Starting intraday price fetch for {} tickers with concurrency {}",
        total_tickers,
        concurrency
    );

    let mut processed = 0;
    let mut successful = 0;
    let mut failed_tickers = Vec::new();

    let results = stream::iter(tickers)
        .enumerate()
        .map(|(idx, ticker)| {
            let db_clone = db.clone();
            async move {
                let result = fetch_prices(db_clone, &ticker, interval, replay).await;
                (idx, ticker, result)
            }
        })
        .buffer_unordered(concurrency)
        .collect::<Vec<_>>()
        .await;

    for (_idx, ticker, result) in results {
        processed += 1;

        match result {
            Ok(_) => {
                successful += 1;
                if processed % progress_interval == 0 || processed == total_tickers {
                    tracing::info!(
                        "Progress: {}/{} processed ({:.1}%), {} successful",
                        processed,
                        total_tickers,
                        (processed as f64 / total_tickers as f64) * 100.0,
                        successful
                    );
                }
            }
            Err(e) => {
                failed_tickers.push(format!("{}:{} - {}", ticker.symbol, ticker.exchange, e));
                tracing::warn!(
                    "Failed to fetch prices for {}:{}: {}",
                    ticker.symbol,
                    ticker.exchange,
                    e
                );
            }
        }
    }

    let failed_count = failed_tickers.len();
    tracing::info!(
        "Intraday processing completed: {}/{} successful ({:.1}% success rate)",
        successful,
        total_tickers,
        (successful as f64 / total_tickers as f64) * 100.0
    );

    if failed_count > 0 {
        tracing::warn!("Failed {} tickers:", failed_count);
        for failure in failed_tickers.iter().take(10) {
            // Show first 10 failures
            tracing::warn!("  {}", failure);
        }
        if failed_count > 10 {
            tracing::warn!("  ... and {} more", failed_count - 10);
        }
    }
    Ok(())
}

pub async fn fetch_intraday_prices_all(
    db: &Database,
    interval: Interval,
    concurrency: usize,
) -> anyhow::Result<()> {
    let tickers = db.get_all_tickers().await?;
    if tickers.is_empty() {
        tracing::warn!("No tickers found in the database");
        return Ok(());
    }

    fetch_intraday_prices(db, &tickers, interval, concurrency, true, true)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch intraday prices: {}", e);
            e
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::finance::db::Database;
    #[tokio::test]
    async fn test() -> anyhow::Result<()> {
        let url = std::env::var("DATABASE_URL").unwrap_or("sqlite::memory:".to_string());
        let db = Database::new(&url).await?;
        let res = db
            .search_tickers_by_field("market_type", "forex", None)
            .await?;
        println!("Found {} tickers", res.len());
        for ticker in res {
            println!(
                "Ticker: {} - {} ({:?})",
                ticker.symbol, ticker.exchange, ticker.market_type
            );
        }

        Ok(())
    }
}
