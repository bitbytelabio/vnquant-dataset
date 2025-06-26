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
        let row = sqlx::query_as!(
            Ticker,
            "SELECT symbol, exchange, description, currency, country, market_type, industry, sector, founded FROM TICKERS WHERE symbol = ?",
            symbol
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn get_ticker(&self, symbol: &str, exchange: &str) -> Result<Option<Ticker>> {
        let row = sqlx::query_as!(
            Ticker,
            "SELECT symbol, exchange, description, currency, country, market_type, industry, sector, founded FROM TICKERS WHERE symbol = ? AND exchange = ?",
            symbol,
            exchange
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
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
        let tickers = sqlx::query_as!(
            Ticker,
            "SELECT symbol, exchange, description, currency, country, market_type, industry, sector, founded FROM TICKERS WHERE exchange = ? ORDER BY symbol",
            exchange
        )
        .fetch_all(&self.pool)
        .await?;

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
    pub async fn upser_ticker(&self, ticker: &SymbolInfo) -> Result<()> {
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
                    .push_bind(ticker.founded);
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
    
        // Filter out invalid OHLCV data before inserting
        let valid_prices: Vec<_> = prices
            .iter()
            .filter(|price| {
                // Check for null/invalid values
                let open = price.open();
                let high = price.high();
                let low = price.low();
                let close = price.close();
                let volume = price.volume();
                
                // Filter out records with null, zero, or negative OHLC values
                let is_valid = !open.is_nan() && !open.is_infinite() && open > 0.0
                    && !high.is_nan() && !high.is_infinite() && high > 0.0
                    && !low.is_nan() && !low.is_infinite() && low > 0.0
                    && !close.is_nan() && !close.is_infinite() && close > 0.0
                    && !volume.is_nan() && !volume.is_infinite() && volume >= 0.0
                    && high >= low // High should be >= low
                    && high >= open && high >= close // High should be >= open and close
                    && low <= open && low <= close; // Low should be <= open and close
                
                if !is_valid {
                    tracing::debug!(
                        "Filtering out invalid OHLCV data for {}:{} at {}: O={}, H={}, L={}, C={}, V={}",
                        ticker.symbol(),
                        ticker.exchange(),
                        price.datetime(),
                        open, high, low, close, volume
                    );
                }
                
                is_valid
            })
            .collect();
    
        if valid_prices.is_empty() {
            tracing::warn!(
                "No valid OHLCV data found for {}:{} after filtering", 
                ticker.symbol(), 
                ticker.exchange()
            );
            return Ok(0);
        }
    
        tracing::debug!(
            "Filtered {} invalid records, inserting {} valid records for {}:{}",
            prices.len() - valid_prices.len(),
            valid_prices.len(),
            ticker.symbol(),
            ticker.exchange()
        );
    
        const BATCH_SIZE: usize = 1000;
        let mut total_affected = 0u64;
    
        for chunk in valid_prices.chunks(BATCH_SIZE) {
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
    pub async fn search_tickers(&self, query: &str, limit: Option<i64>) -> Result<Vec<Ticker>> {
        let limit = limit.unwrap_or(50);
        
        let tickers = sqlx::query_as!(
            Ticker,
            r#"
            SELECT t.symbol, t.exchange, t.description, t.currency, t.country, 
                   t.market_type, t.industry, t.sector, t.founded
            FROM tickers_fts 
            JOIN TICKERS t ON tickers_fts.rowid = t.rowid
            WHERE tickers_fts MATCH ?
            ORDER BY bm25(tickers_fts)
            LIMIT ?
            "#,
            query,
            limit
        )
        .fetch_all(&self.pool)
        .await?;


        Ok(tickers)
    }

    /// Search tickers with additional filtering by exchange
    pub async fn search_tickers_by_exchange(
        &self, 
        query: &str, 
        exchange: &str, 
        limit: Option<i64>
    ) -> Result<Vec<Ticker>> {
        let limit = limit.unwrap_or(50);
        
        let rows = sqlx::query_as!(
            Ticker,
            r#"
            SELECT t.symbol, t.exchange, t.description, t.currency, t.country, 
                   t.market_type, t.industry, t.sector, t.founded
            FROM tickers_fts 
            JOIN TICKERS t ON tickers_fts.rowid = t.rowid
            WHERE tickers_fts MATCH ? AND t.exchange = ?
            ORDER BY bm25(tickers_fts)
            LIMIT ?
            "#,
            query,
            exchange,
            limit
        )
        .fetch_all(&self.pool)
        .await?;
    
        Ok(rows)
    }

    /// Search tickers by specific field (symbol, description, industry, or sector)
    pub async fn search_tickers_by_field(
        &self, 
        field: &str, 
        query: &str, 
        limit: Option<i64>
    ) -> Result<Vec<Ticker>> {
        let limit = limit.unwrap_or(50);
        
        // Validate field name to prevent SQL injection
        let valid_fields = ["symbol", "description", "industry", "sector"];
        if !valid_fields.contains(&field) {
            return Err(anyhow::anyhow!("Invalid field name: {}", field));
        }

        let search_query = format!("{}: {}", field, query);
        
        let rows = sqlx::query_as!(
            Ticker,
            r#"
            SELECT t.symbol, t.exchange, t.description, t.currency, t.country, 
                   t.market_type, t.industry, t.sector, t.founded
            FROM tickers_fts 
            JOIN TICKERS t ON tickers_fts.rowid = t.rowid
            WHERE tickers_fts MATCH ?
            ORDER BY bm25(tickers_fts)
            LIMIT ?
            "#,
            search_query,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }


    /// Rebuild the FTS index (useful for maintenance)
    pub async fn rebuild_search_index(&self) -> Result<()> {
        // Clear existing FTS data
        sqlx::query("DELETE FROM tickers_fts").execute(&self.pool).await?;
        
        // Repopulate FTS table
        sqlx::query!(
            "INSERT INTO tickers_fts(symbol, description, industry, sector) SELECT symbol, description, industry, sector FROM TICKERS"
        )
        .execute(&self.pool)
        .await?;

        // Optimize the FTS index
        sqlx::query("INSERT INTO tickers_fts(tickers_fts) VALUES('optimize')")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

}
