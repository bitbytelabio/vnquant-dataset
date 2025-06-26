-- Add migration script here
-- Create FTS virtual table for full-text search on tickers
CREATE VIRTUAL TABLE IF NOT EXISTS tickers_fts USING fts5(
    symbol,
    description,
    industry,
    sector,
    content='TICKERS',
    content_rowid='rowid'
);

-- Populate FTS table with existing data
INSERT INTO tickers_fts(symbol, description, industry, sector)
SELECT symbol, description, industry, sector FROM TICKERS;

-- Create triggers to keep FTS table in sync with TICKERS table
CREATE TRIGGER IF NOT EXISTS tickers_fts_insert 
    AFTER INSERT ON TICKERS
BEGIN
    INSERT INTO tickers_fts(rowid, symbol, description, industry, sector)
    VALUES (NEW.rowid, NEW.symbol, NEW.description, NEW.industry, NEW.sector);
END;

CREATE TRIGGER IF NOT EXISTS tickers_fts_delete 
    AFTER DELETE ON TICKERS
BEGIN
    DELETE FROM tickers_fts WHERE rowid = OLD.rowid;
END;

CREATE TRIGGER IF NOT EXISTS tickers_fts_update 
    AFTER UPDATE ON TICKERS
BEGIN
    UPDATE tickers_fts SET 
        symbol = NEW.symbol,
        description = NEW.description,
        industry = NEW.industry,
        sector = NEW.sector
    WHERE rowid = NEW.rowid;
END;
