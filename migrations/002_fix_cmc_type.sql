-- Fix cmc column type to match Rust f64 type
ALTER TABLE cards ALTER COLUMN cmc TYPE DOUBLE PRECISION;
