-- Add migration script here
-- Migration to update FTS table to include all searchable columns

-- Drop existing FTS table
DROP TABLE IF EXISTS tickers_fts;

-- Create new FTS table with all searchable columns
CREATE VIRTUAL TABLE tickers_fts USING fts5(
    symbol,
    exchange, 
    description,
    currency,
    country,
    market_type,
    industry,
    sector,
    content='TICKERS',
    content_rowid='rowid'
);

-- Populate the FTS table with all data
INSERT INTO tickers_fts(symbol, exchange, description, currency, country, market_type, industry, sector)
SELECT symbol, exchange, description, currency, country, market_type, industry, sector 
FROM TICKERS;

-- Create trigger to keep FTS in sync with TICKERS table
CREATE TRIGGER IF NOT EXISTS tickers_fts_insert AFTER INSERT ON TICKERS BEGIN
    INSERT INTO tickers_fts(rowid, symbol, exchange, description, currency, country, market_type, industry, sector)
    VALUES (NEW.rowid, NEW.symbol, NEW.exchange, NEW.description, NEW.currency, NEW.country, NEW.market_type, NEW.industry, NEW.sector);
END;

CREATE TRIGGER IF NOT EXISTS tickers_fts_delete AFTER DELETE ON TICKERS BEGIN
    INSERT INTO tickers_fts(tickers_fts, rowid, symbol, exchange, description, currency, country, market_type, industry, sector)
    VALUES ('delete', OLD.rowid, OLD.symbol, OLD.exchange, OLD.description, OLD.currency, OLD.country, OLD.market_type, OLD.industry, OLD.sector);
END;

CREATE TRIGGER IF NOT EXISTS tickers_fts_update AFTER UPDATE ON TICKERS BEGIN
    INSERT INTO tickers_fts(tickers_fts, rowid, symbol, exchange, description, currency, country, market_type, industry, sector)
    VALUES ('delete', OLD.rowid, OLD.symbol, OLD.exchange, OLD.description, OLD.currency, OLD.country, OLD.market_type, OLD.industry, OLD.sector);
    INSERT INTO tickers_fts(rowid, symbol, exchange, description, currency, country, market_type, industry, sector)
    VALUES (NEW.rowid, NEW.symbol, NEW.exchange, NEW.description, NEW.currency, NEW.country, NEW.market_type, NEW.industry, NEW.sector);
END;