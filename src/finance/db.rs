use crate::finance::models::*;
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use tradingview::{Interval, MarketSymbol, OHLCV, SymbolInfo};

#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

#[bon::bon]
impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;

        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn get_pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn close(&self) -> Result<()> {
        self.pool.close().await;
        Ok(())
    }

    pub async fn execute(&self, query: &str) -> Result<()> {
        sqlx::query(query).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn get_ticker_by_symbol(&self, symbol: &str) -> Result<Option<Ticker>> {
        let row = sqlx::query!(
            "SELECT symbol, exchange, description, currency, country, market_type, industry, sector, founded, created_at, updated_at FROM TICKERS WHERE symbol = ?",
            symbol
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Ticker {
                symbol: row.symbol,
                exchange: row.exchange,
                description: row.description,
                currency: row.currency,
                country: row.country,
                market_type: row.market_type,
                industry: row.industry,
                sector: row.sector,
                founded: row.founded,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_ticker(&self, symbol: &str, exchange: &str) -> Result<Option<Ticker>> {
        let row = sqlx::query!(
            "SELECT symbol, exchange, description, currency, country, market_type, industry, sector, founded, created_at, updated_at FROM TICKERS WHERE symbol = ? AND exchange = ?",
            symbol,
            exchange
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Ticker {
                symbol: row.symbol,
                exchange: row.exchange,
                description: row.description,
                currency: row.currency,
                country: row.country,
                market_type: row.market_type,
                industry: row.industry,
                sector: row.sector,
                founded: row.founded,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_all_tickers(&self) -> Result<Vec<Ticker>> {
        let rows = sqlx::query!(
            "SELECT symbol, exchange, description, currency, country, market_type, industry, sector, founded FROM tickers ORDER BY symbol"
        )
        .fetch_all(&self.pool)
        .await?;

        let tickers = rows
            .into_iter()
            .map(|row| Ticker {
                symbol: row.symbol,
                exchange: row.exchange,
                description: row.description,
                currency: row.currency,
                country: row.country,
                market_type: row.market_type,
                industry: row.industry,
                sector: row.sector,
                founded: row.founded,
            })
            .collect();

        Ok(tickers)
    }

    pub async fn get_tickers_by_exchange(&self, exchange: &str) -> Result<Vec<Ticker>> {
        let rows = sqlx::query!(
            "SELECT symbol, exchange, description, currency, country, market_type, industry, sector, founded FROM TICKERS WHERE exchange = ? ORDER BY symbol",
            exchange
        )
        .fetch_all(&self.pool)
        .await?;

        let tickers = rows
            .into_iter()
            .map(|row| Ticker {
                symbol: row.symbol,
                exchange: row.exchange,
                description: row.description,
                currency: row.currency,
                country: row.country,
                market_type: row.market_type,
                industry: row.industry,
                sector: row.sector,
                founded: row.founded,
            })
            .collect();

        Ok(tickers)
    }

    pub async fn ticker_exists(&self, symbol: &str, exchange: &str) -> Result<bool> {
        let count = sqlx::query!(
            "SELECT COUNT(*) as count FROM TICKERS WHERE symbol = ? AND exchange = ?",
            symbol,
            exchange
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count.count > 0)
    }

    // Improved INSERT with upsert capability
    pub async fn update_ticker(&self, ticker: &SymbolInfo) -> Result<()> {
        let ticker = Ticker {
            symbol: ticker.symbol().to_string(),
            exchange: ticker.exchange().to_string(),
            description: Some(ticker.description.clone()),
            currency: Some(ticker.currency_code.clone()),
            country: None, // Country is not available in SymbolInfo, need to update this later
            market_type: Some(ticker.market_type.clone()),
            industry: Some(ticker.industry.clone()),
            sector: Some(ticker.sector.clone()),
            founded: Some(ticker.founded.into()),
        };
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query!(
            "INSERT INTO TICKERS (symbol, exchange, description, currency, country, market_type, industry, sector, founded) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(symbol, exchange) DO UPDATE SET description = excluded.description, currency = excluded.currency, country = excluded.country, market_type = excluded.market_type, industry = excluded.industry, sector = excluded.sector, founded = excluded.founded",
            ticker.symbol,
            ticker.exchange,
            ticker.description,
            ticker.currency,
            ticker.country,
            ticker.market_type,
            ticker.industry,
            ticker.sector,
            ticker.founded
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        tracing::info!(
            "Upserted ticker {} on exchange {}: {} rows affected",
            ticker.symbol,
            ticker.exchange,
            result.rows_affected()
        );

        Ok(())
    }

    // Batch upsert with better performance
    pub async fn upsert_tickers(&self, tickers: &[Ticker]) -> Result<u64> {
        if tickers.is_empty() {
            return Ok(0);
        }

        const BATCH_SIZE: usize = 1000;
        let mut total_affected = 0u64;

        for chunk in tickers.chunks(BATCH_SIZE) {
            let mut tx = self.pool.begin().await?;

            let mut query_builder = sqlx::QueryBuilder::new(
                "INSERT INTO tickers (symbol, exchange, description, currency, country, market_type, industry, sector, founded) ",
            );

            query_builder.push_values(chunk, |mut b, ticker| {
                b.push_bind(&ticker.symbol)
                    .push_bind(&ticker.exchange)
                    .push_bind(&ticker.description)
                    .push_bind(&ticker.currency)
                    .push_bind(&ticker.country)
                    .push_bind(&ticker.market_type)
                    .push_bind(&ticker.industry)
                    .push_bind(&ticker.sector)
                    .push_bind(&ticker.founded);
            });

            query_builder.push(" ON CONFLICT(symbol, exchange) DO UPDATE SET ");
            query_builder.push("description = excluded.description, ");
            query_builder.push("currency = excluded.currency, ");
            query_builder.push("country = excluded.country, ");
            query_builder.push("market_type = excluded.market_type, ");
            query_builder.push("industry = excluded.industry, ");
            query_builder.push("sector = excluded.sector, ");
            query_builder.push("founded = excluded.founded");

            let query = query_builder.build();
            let result = query.execute(&mut *tx).await?;
            total_affected += result.rows_affected();

            tx.commit().await?;
        }

        Ok(total_affected)
    }

    // DELETE operations
    pub async fn delete_ticker(&self, symbol: &str, exchange: &str) -> Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM TICKERS WHERE symbol = ? AND exchange = ?",
            symbol,
            exchange
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_tickers_by_exchange(&self, exchange: &str) -> Result<u64> {
        let result = sqlx::query!("DELETE FROM tickers WHERE exchange = ?", exchange)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    // Updated to use Ticker model for consistency
    pub async fn update_ticker_info(&self, ticker: &Ticker) -> Result<bool> {
        let result = sqlx::query!(
            "UPDATE TICKERS SET description = ?, currency = ?, country = ?, market_type = ?, industry = ?, sector = ?, founded = ? WHERE symbol = ? AND exchange = ?",
            ticker.description,
            ticker.currency,
            ticker.country,
            ticker.market_type,
            ticker.industry,
            ticker.sector,
            ticker.founded,
            ticker.symbol,
            ticker.exchange
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn get_ticker_count(&self) -> Result<i64> {
        let count = sqlx::query!("SELECT COUNT(*) as count FROM TICKERS")
            .fetch_one(&self.pool)
            .await?;

        Ok(count.count)
    }

    pub async fn upsert_prices(
        &self,
        ticker: &impl MarketSymbol,
        interval: Interval,
        prices: &[impl OHLCV],
    ) -> Result<u64> {
        if prices.is_empty() {
            return Ok(0);
        }

        const BATCH_SIZE: usize = 1000;
        let mut total_affected = 0u64;

        for chunk in prices.chunks(BATCH_SIZE) {
            let mut tx = self.pool.begin().await?;

            let mut query_builder = sqlx::QueryBuilder::new(
                "INSERT OR REPLACE INTO OHLCV (symbol, exchange, interval, timestamp, open, high, low, close, volume) ",
            );

            query_builder.push_values(chunk, |mut b, price| {
                b.push_bind(ticker.symbol())
                    .push_bind(ticker.exchange())
                    .push_bind(interval.to_string())
                    .push_bind(price.datetime())
                    .push_bind(price.open())
                    .push_bind(price.high())
                    .push_bind(price.low())
                    .push_bind(price.close())
                    .push_bind(price.volume());
            });

            let query = query_builder.build();
            let result = query.execute(&mut *tx).await?;
            total_affected += result.rows_affected();

            tx.commit().await?;
        }

        Ok(total_affected)
    }

    #[builder]
    pub async fn get_prices(
        &self,
        ticker: &Ticker,
        interval: Interval,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Result<Vec<Candle>> {
        let mut query = sqlx::QueryBuilder::new(
            "SELECT timestamp, open, high, low, close, volume FROM OHLCV WHERE symbol = ",
        );
        query.push_bind(&ticker.symbol);
        query.push(" AND exchange = ");
        query.push_bind(&ticker.exchange);
        query.push(" AND interval = ");
        query.push_bind(interval.to_string());

        if let Some(start_date) = start {
            query.push(" AND timestamp >= ");
            query.push_bind(start_date);
        }

        if let Some(end_date) = end {
            query.push(" AND timestamp <= ");
            query.push_bind(end_date);
        }

        query.push(" ORDER BY timestamp ASC");

        let rows = query
            .build_query_as::<(chrono::DateTime<Utc>, f64, f64, f64, f64, f64)>()
            .fetch_all(&self.pool)
            .await?;

        let candles = rows
            .into_iter()
            .map(|row| Candle {
                timestamp: row.0,
                open: row.1,
                high: row.2,
                low: row.3,
                close: row.4,
                volume: row.5,
            })
            .collect();

        Ok(candles)
    }
}
