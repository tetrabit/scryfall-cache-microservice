-- Scryfall Cache Database Schema

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Cards table: stores all Scryfall card data
CREATE TABLE IF NOT EXISTS cards (
    id UUID PRIMARY KEY,
    oracle_id UUID,
    name TEXT NOT NULL,
    mana_cost TEXT,
    cmc DECIMAL,
    type_line TEXT,
    oracle_text TEXT,
    colors TEXT[],
    color_identity TEXT[],
    set_code TEXT,
    set_name TEXT,
    collector_number TEXT,
    rarity TEXT,
    power TEXT,
    toughness TEXT,
    loyalty TEXT,
    keywords TEXT[],
    prices JSONB,
    image_uris JSONB,
    card_faces JSONB,
    legalities JSONB,
    released_at DATE,
    raw_json JSONB NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_cards_name ON cards USING gin(to_tsvector('english', name));
CREATE INDEX IF NOT EXISTS idx_cards_oracle_text ON cards USING gin(to_tsvector('english', COALESCE(oracle_text, '')));
CREATE INDEX IF NOT EXISTS idx_cards_type_line ON cards USING gin(to_tsvector('english', COALESCE(type_line, '')));
CREATE INDEX IF NOT EXISTS idx_cards_colors ON cards USING gin(colors);
CREATE INDEX IF NOT EXISTS idx_cards_color_identity ON cards USING gin(color_identity);
CREATE INDEX IF NOT EXISTS idx_cards_set_code ON cards(set_code);
CREATE INDEX IF NOT EXISTS idx_cards_cmc ON cards(cmc);
CREATE INDEX IF NOT EXISTS idx_cards_oracle_id ON cards(oracle_id);
CREATE INDEX IF NOT EXISTS idx_cards_rarity ON cards(rarity);
CREATE INDEX IF NOT EXISTS idx_cards_keywords ON cards USING gin(keywords);
CREATE INDEX IF NOT EXISTS idx_cards_released_at ON cards(released_at);

-- Query cache table: stores parsed query results
CREATE TABLE IF NOT EXISTS query_cache (
    query_hash TEXT PRIMARY KEY,
    query_text TEXT NOT NULL,
    result_ids UUID[] NOT NULL,
    total_cards INTEGER NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    last_accessed TIMESTAMP DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_query_cache_last_accessed ON query_cache(last_accessed);

-- Bulk data metadata table: tracks bulk data imports
CREATE TABLE IF NOT EXISTS bulk_data_metadata (
    id SERIAL PRIMARY KEY,
    bulk_type TEXT NOT NULL,
    download_uri TEXT NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    imported_at TIMESTAMP DEFAULT NOW(),
    total_cards INTEGER NOT NULL,
    file_size_bytes BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_bulk_data_imported_at ON bulk_data_metadata(imported_at DESC);

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Trigger for cards table
DROP TRIGGER IF EXISTS update_cards_updated_at ON cards;
CREATE TRIGGER update_cards_updated_at
    BEFORE UPDATE ON cards
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
