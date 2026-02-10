use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::models::card::Card;

/// Insert a batch of cards into the database
pub async fn insert_cards_batch(pool: &PgPool, cards: &[Card]) -> Result<()> {
    if cards.is_empty() {
        return Ok(());
    }

    let mut transaction = pool.begin().await.context("Failed to begin transaction")?;

    // Insert in chunks to avoid enormous SQL statements while still reducing per-row overhead.
    const CHUNK_SIZE: usize = 250;
    for chunk in cards.chunks(CHUNK_SIZE) {
        let mut builder = QueryBuilder::<Postgres>::new(
            r#"
            INSERT INTO cards (
                id, oracle_id, name, mana_cost, cmc, type_line, oracle_text,
                colors, color_identity, set_code, set_name, collector_number,
                rarity, power, toughness, loyalty, keywords, prices, image_uris,
                card_faces, legalities, released_at, raw_json
            )
            "#,
        );

        builder.push_values(chunk, |mut b, card| {
            b.push_bind(card.id)
                .push_bind(card.oracle_id)
                .push_bind(&card.name)
                .push_bind(&card.mana_cost)
                .push_bind(card.cmc)
                .push_bind(&card.type_line)
                .push_bind(&card.oracle_text)
                .push_bind(&card.colors)
                .push_bind(&card.color_identity)
                .push_bind(&card.set_code)
                .push_bind(&card.set_name)
                .push_bind(&card.collector_number)
                .push_bind(&card.rarity)
                .push_bind(&card.power)
                .push_bind(&card.toughness)
                .push_bind(&card.loyalty)
                .push_bind(&card.keywords)
                .push_bind(&card.prices)
                .push_bind(&card.image_uris)
                .push_bind(&card.card_faces)
                .push_bind(&card.legalities)
                .push_bind(card.released_at)
                .push_bind(&card.raw_json);
        });

        builder.push(
            r#"
            ON CONFLICT (id) DO UPDATE SET
                oracle_id = EXCLUDED.oracle_id,
                name = EXCLUDED.name,
                mana_cost = EXCLUDED.mana_cost,
                cmc = EXCLUDED.cmc,
                type_line = EXCLUDED.type_line,
                oracle_text = EXCLUDED.oracle_text,
                colors = EXCLUDED.colors,
                color_identity = EXCLUDED.color_identity,
                set_code = EXCLUDED.set_code,
                set_name = EXCLUDED.set_name,
                collector_number = EXCLUDED.collector_number,
                rarity = EXCLUDED.rarity,
                power = EXCLUDED.power,
                toughness = EXCLUDED.toughness,
                loyalty = EXCLUDED.loyalty,
                keywords = EXCLUDED.keywords,
                prices = EXCLUDED.prices,
                image_uris = EXCLUDED.image_uris,
                card_faces = EXCLUDED.card_faces,
                legalities = EXCLUDED.legalities,
                released_at = EXCLUDED.released_at,
                raw_json = EXCLUDED.raw_json,
                updated_at = NOW()
            "#,
        );

        builder
            .build()
            .execute(&mut *transaction)
            .await
            .context("Failed to insert card batch")?;
    }

    transaction
        .commit()
        .await
        .context("Failed to commit transaction")?;

    Ok(())
}

/// Get a card by ID
pub async fn get_card_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Card>> {
    let card = sqlx::query_as::<_, Card>("SELECT * FROM cards WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch card by ID")?;

    Ok(card)
}

/// Get multiple cards by IDs
pub async fn get_cards_by_ids(pool: &PgPool, ids: &[Uuid]) -> Result<Vec<Card>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    // Use array binding to avoid dynamic SQL and keep the statement cacheable.
    // This is typically faster than building a large IN (...) list.
    let cards = sqlx::query_as::<_, Card>("SELECT * FROM cards WHERE id = ANY($1)")
        .bind(ids)
        .fetch_all(pool)
        .await
        .context("Failed to fetch cards by IDs")?;

    Ok(cards)
}

/// Search cards by name (fuzzy match)
pub async fn search_cards_by_name(pool: &PgPool, name: &str, limit: i64) -> Result<Vec<Card>> {
    let cards = sqlx::query_as::<_, Card>(
        r#"
        SELECT * FROM cards
        WHERE to_tsvector('english', name) @@ plainto_tsquery('english', $1)
        ORDER BY name
        LIMIT $2
        "#,
    )
    .bind(name)
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("Failed to search cards by name")?;

    Ok(cards)
}

/// Autocomplete card names by prefix (case-insensitive)
/// Uses the existing idx_cards_name GIN index for fast prefix matching
pub async fn autocomplete_card_names(
    pool: &PgPool,
    prefix: &str,
    limit: i64,
) -> Result<Vec<String>> {
    // Use ILIKE for case-insensitive prefix matching
    // The idx_cards_name GIN index can be used for prefix searches in PostgreSQL
    let pattern = format!("{}%", prefix);

    let names: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT DISTINCT name
        FROM cards
        WHERE name ILIKE $1
        ORDER BY name
        LIMIT $2
        "#,
    )
    .bind(&pattern)
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("Failed to autocomplete card names")?;

    Ok(names.into_iter().map(|(name,)| name).collect())
}

/// Store query cache entry
pub async fn store_query_cache(
    pool: &PgPool,
    query_hash: &str,
    card_ids: &[Uuid],
    ttl_hours: i32,
) -> Result<()> {
    let card_ids_json = serde_json::to_string(card_ids).context("Failed to serialize card IDs")?;

    sqlx::query(
        r#"
        INSERT INTO query_cache (query_hash, result_ids, ttl_hours)
        VALUES ($1, $2, $3)
        ON CONFLICT (query_hash) DO UPDATE SET
            result_ids = EXCLUDED.result_ids,
            ttl_hours = EXCLUDED.ttl_hours,
            last_accessed = NOW()
        "#,
    )
    .bind(query_hash)
    .bind(&card_ids_json)
    .bind(ttl_hours)
    .execute(pool)
    .await
    .context("Failed to store query cache")?;

    Ok(())
}

/// Get query cache entry
pub async fn get_query_cache(pool: &PgPool, query_hash: &str) -> Result<Option<(Vec<Uuid>, i32)>> {
    let result: Option<(String, i32)> = sqlx::query_as(
        r#"
        UPDATE query_cache
        SET last_accessed = NOW()
        WHERE query_hash = $1
        RETURNING result_ids, ttl_hours
        "#,
    )
    .bind(query_hash)
    .fetch_optional(pool)
    .await
    .context("Failed to get query cache")?;

    if let Some((card_ids_json, ttl_hours)) = result {
        let card_ids: Vec<Uuid> =
            serde_json::from_str(&card_ids_json).context("Failed to deserialize card IDs")?;
        Ok(Some((card_ids, ttl_hours)))
    } else {
        Ok(None)
    }
}

/// Record bulk data import
pub async fn record_bulk_import(pool: &PgPool, total_cards: i32, source: &str) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO bulk_data_metadata (total_cards, source, imported_at)
        VALUES ($1, $2, NOW())
        "#,
    )
    .bind(total_cards)
    .bind(source)
    .execute(pool)
    .await
    .context("Failed to record bulk import")?;

    Ok(())
}

/// Clean old query cache entries
pub async fn clean_old_cache_entries(pool: &PgPool, hours: i32) -> Result<u64> {
    let result = sqlx::query(
        r#"
        DELETE FROM query_cache
        WHERE last_accessed < NOW() - INTERVAL '1 hour' * $1
        "#,
    )
    .bind(hours)
    .execute(pool)
    .await
    .context("Failed to clean old cache entries")?;

    Ok(result.rows_affected())
}

/// Execute a raw SQL query and return Card results
pub async fn execute_raw_query(pool: &PgPool, sql: &str, params: &[String]) -> Result<Vec<Card>> {
    let mut query_builder = sqlx::query_as::<_, Card>(sql);

    // Bind all parameters
    for param in params {
        query_builder = query_builder.bind(param.clone());
    }

    let cards = query_builder
        .fetch_all(pool)
        .await
        .context("Failed to execute raw query")?;

    Ok(cards)
}

/// Execute a COUNT query and return the result
pub async fn count_query(pool: &PgPool, sql: &str, params: &[String]) -> Result<usize> {
    let mut query_builder = sqlx::query_scalar::<_, i64>(sql);

    // Bind all parameters
    for param in params {
        query_builder = query_builder.bind(param.clone());
    }

    let count = query_builder
        .fetch_one(pool)
        .await
        .context("Failed to execute COUNT query")?;

    Ok(count as usize)
}

/// Check if bulk data is loaded
pub async fn check_bulk_data_loaded(pool: &PgPool) -> Result<bool> {
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cards")
        .fetch_one(pool)
        .await
        .context("Failed to check if bulk data is loaded")?;

    Ok(result.0 > 0)
}

/// Get the timestamp of the last bulk import
pub async fn get_last_bulk_import(pool: &PgPool) -> Result<Option<chrono::NaiveDateTime>> {
    let result: Option<(chrono::NaiveDateTime,)> = sqlx::query_as(
        "SELECT imported_at FROM bulk_data_metadata ORDER BY imported_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .context("Failed to get last bulk import timestamp")?;

    Ok(result.map(|r| r.0))
}

/// Get the total count of cards in the database
pub async fn get_card_count(pool: &PgPool) -> Result<i64> {
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cards")
        .fetch_one(pool)
        .await
        .context("Failed to get card count")?;

    Ok(result.0)
}

/// Get the total count of query cache entries
pub async fn get_cache_entry_count(pool: &PgPool) -> Result<i64> {
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM query_cache")
        .fetch_one(pool)
        .await
        .context("Failed to get cache entry count")?;

    Ok(result.0)
}
