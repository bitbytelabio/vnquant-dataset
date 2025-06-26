-- Add migration script here
CREATE TABLE IF NOT EXISTS TICKERS (
    symbol VARCHAR(10) NOT NULL,
    exchange VARCHAR(10) NOT NULL,
    description TEXT,
    currency VARCHAR(3),
    country VARCHAR(50),
    market_type VARCHAR(20),
    industry VARCHAR(50),
    sector VARCHAR(50),
    founded INTEGER, 
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (symbol, exchange)
);

-- Create trigger for updated_at since SQLite doesn't support ON UPDATE CURRENT_TIMESTAMP
CREATE TRIGGER IF NOT EXISTS tickers_updated_at 
    AFTER UPDATE ON TICKERS
BEGIN
    UPDATE TICKERS SET updated_at = CURRENT_TIMESTAMP WHERE symbol = NEW.symbol AND exchange = NEW.exchange;
END;

CREATE TABLE IF NOT EXISTS OHLCV (
    symbol VARCHAR(10) NOT NULL,
    exchange VARCHAR(10) NOT NULL,
    interval VARCHAR(10) NOT NULL,
    timestamp DATETIME NOT NULL,
    open REAL NOT NULL,
    high REAL NOT NULL,
    low REAL NOT NULL,
    close REAL NOT NULL,
    volume REAL NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    PRIMARY KEY (symbol, exchange, interval, timestamp),
    FOREIGN KEY (symbol, exchange) REFERENCES TICKERS(symbol, exchange) ON DELETE CASCADE
) WITHOUT ROWID; -- More efficient for composite primary keys in read-heavy scenarios

-- Optimized indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_ohlcv_symbol_interval_timestamp ON OHLCV(symbol, interval, timestamp DESC); -- Time series queries
CREATE INDEX IF NOT EXISTS idx_ohlcv_exchange_timestamp ON OHLCV(exchange, timestamp DESC); -- Exchange-wide queries
CREATE INDEX IF NOT EXISTS idx_ohlcv_timestamp_desc ON OHLCV(timestamp DESC); -- Latest data queries
CREATE INDEX IF NOT EXISTS idx_ohlcv_symbol_exchange_interval ON OHLCV(symbol, exchange, interval); -- Covering index for metadata

CREATE TABLE IF NOT EXISTS TECHNICAL_INDICATORS (
    symbol VARCHAR(10) NOT NULL,
    exchange VARCHAR(10) NOT NULL,
    interval VARCHAR(10) NOT NULL,
    timestamp DATETIME NOT NULL,
    indicator_type VARCHAR(20) NOT NULL,
    value REAL,
    metadata TEXT, -- JSON for additional indicator-specific data
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    PRIMARY KEY (symbol, exchange, interval, timestamp, indicator_type),
    FOREIGN KEY (symbol, exchange) REFERENCES TICKERS(symbol, exchange) ON DELETE CASCADE
) WITHOUT ROWID; -- Remove surrogate key for better read performance

-- Optimized indexes for technical indicators
CREATE INDEX IF NOT EXISTS idx_tech_symbol_type_timestamp ON TECHNICAL_INDICATORS(symbol, indicator_type, timestamp DESC); -- Specific indicator queries
CREATE INDEX IF NOT EXISTS idx_tech_type_timestamp ON TECHNICAL_INDICATORS(indicator_type, timestamp DESC); -- Cross-symbol indicator analysis
CREATE INDEX IF NOT EXISTS idx_tech_symbol_interval_timestamp ON TECHNICAL_INDICATORS(symbol, interval, timestamp DESC); -- Time series by interval
CREATE INDEX IF NOT EXISTS idx_tech_timestamp_desc ON TECHNICAL_INDICATORS(timestamp DESC); -- Latest indicators

-- Optional: Create materialized view for latest OHLCV data (most recent timestamp per symbol/interval)
CREATE VIEW IF NOT EXISTS latest_ohlcv AS
SELECT symbol, exchange, interval, 
       MAX(timestamp) as latest_timestamp,
       open, high, low, close, volume
FROM OHLCV 
GROUP BY symbol, exchange, interval;

-- Optional: Create view for commonly queried technical indicators
CREATE VIEW IF NOT EXISTS latest_indicators AS
SELECT symbol, exchange, interval,
       MAX(timestamp) as latest_timestamp,
       indicator_type, value, metadata
FROM TECHNICAL_INDICATORS
GROUP BY symbol, exchange, interval, indicator_type;