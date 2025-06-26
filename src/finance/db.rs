use crate::finance::models::*;
use anyhow::Result;
use sqlx::SqlitePool;

#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

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
    pub async fn insert_or_update_ticker(&self, ticker: &Ticker) -> Result<()> {
        sqlx::query!(
            "INSERT INTO TICKERS (symbol, exchange, description, currency, country, market_type, industry, sector, founded) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(symbol, exchange) DO UPDATE SET
                description = excluded.description,
                currency = excluded.currency,
                country = excluded.country,
                market_type = excluded.market_type,
                industry = excluded.industry,
                sector = excluded.sector,
                founded = excluded.founded",
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
        .execute(&self.pool)
        .await?;
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
}
