-- Migration to add foreign key constraint from TECHNICAL_INDICATORS to OHLCV

-- SQLite doesn't support ALTER TABLE ADD CONSTRAINT directly
-- We need to recreate the table with the additional foreign key constraint

-- Step 1: Drop the dependent view first
DROP VIEW IF EXISTS latest_indicators;

-- Step 2: Create new table with both foreign key constraints
CREATE TABLE TECHNICAL_INDICATORS_new (
    symbol VARCHAR(10) NOT NULL,
    exchange VARCHAR(10) NOT NULL,
    interval VARCHAR(10) NOT NULL,
    timestamp DATETIME NOT NULL,
    indicator_type VARCHAR(20) NOT NULL,
    value REAL,
    metadata TEXT, -- JSON for additional indicator-specific data
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    PRIMARY KEY (symbol, exchange, interval, timestamp, indicator_type),
    FOREIGN KEY (symbol, exchange) REFERENCES TICKERS(symbol, exchange) ON DELETE CASCADE,
    FOREIGN KEY (symbol, exchange, interval, timestamp) REFERENCES OHLCV(symbol, exchange, interval, timestamp) ON DELETE CASCADE
) WITHOUT ROWID;

-- Step 3: Copy data from old table to new table
INSERT INTO TECHNICAL_INDICATORS_new 
SELECT symbol, exchange, interval, timestamp, indicator_type, value, metadata, created_at 
FROM TECHNICAL_INDICATORS;

-- Step 4: Drop old table
DROP TABLE TECHNICAL_INDICATORS;

-- Step 5: Rename new table
ALTER TABLE TECHNICAL_INDICATORS_new RENAME TO TECHNICAL_INDICATORS;

-- Step 6: Recreate indexes
CREATE INDEX IF NOT EXISTS idx_tech_symbol_type_timestamp ON TECHNICAL_INDICATORS(symbol, indicator_type, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_tech_type_timestamp ON TECHNICAL_INDICATORS(indicator_type, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_tech_symbol_interval_timestamp ON TECHNICAL_INDICATORS(symbol, interval, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_tech_timestamp_desc ON TECHNICAL_INDICATORS(timestamp DESC);

-- Step 7: Recreate the view after the table is properly renamed
CREATE VIEW IF NOT EXISTS latest_indicators AS
SELECT symbol, exchange, interval,
       MAX(timestamp) as latest_timestamp,
       indicator_type, value, metadata
FROM TECHNICAL_INDICATORS
GROUP BY symbol, exchange, interval, indicator_type;