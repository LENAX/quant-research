CREATE TABLE market_quotation (id INTEGER NOT NULL, uuid VARCHAR(32) NOT NULL, source VARCHAR(32) NOT NULL,quotation TEXT NOT NULL, create_time DATETIME,PRIMARY KEY (id));
CREATE INDEX ix_market_quote_uuid ON market_quotation (uuid);
CREATE INDEX ix_market_quote_id ON market_quotation (id);
CREATE INDEX ix_market_quote_time ON market_quotation (create_time);
CREATE INDEX ix_market_quote_source on market_quotation (source);