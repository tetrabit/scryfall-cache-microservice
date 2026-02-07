use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::card::Card;

/// Insert a batch of cards into the database
pub async fn insert_cards_batch(pool: &PgPool, cards: &[Card]) -> Result<()> {
    if cards.is_empty() {
        return Ok(());
    }

    let mut transaction = pool.begin().await.context("Failed to begin transaction")?;

    for card in cards {
        sqlx::query(
            r#"
            INSERT INTO cards (
                id, oracle_id, name, mana_cost, cmc, type_line, oracle_text,
                colors, color_identity, set_code, set_name, collector_number,
                rarity, power, toughness, loyalty, keywords, prices, image_uris,
                card_faces, legalities, released_at, raw_json
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
                $14, $15, $16, $17, $18, $19, $20, $21, $22, $23
            )
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
        )
        .bind(&card.id)
        .bind(&card.oracle_id)
        .bind(&card.name)
        .bind(&card.mana_cost)
        .bind(&card.cmc)
        .bind(&card.type_line)
        .bind(&card.oracle_text)
        .bind(&card.colors)
        .bind(&card.color_identity)
        .bind(&card.set_code)
        .bind(&card.set_name)
        .bind(&card.collector_number)
        .bind(&card.rarity)
        .bind(&card.power)
        .bind(&card.toughness)
        .bind(&card.loyalty)
        .bind(&card.keywords)
        .bind(&card.prices)
        .bind(&card.image_uris)
        .bind(&card.card_faces)
        .bind(&card.legalities)
        .bind(&card.released_at)
        .bind(&card.raw_json)
        .execute(&mut *transaction)
        .await
        .context("Failed to insert card")?;
    }

    transaction.commit().await.context("Failed to commit transaction")?;

    Ok(())
}

/// Get a card by ID
pub async fn get_card_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Card>> {
    let card = sqlx::query_as::<_, Card>(
        "SELECT * FROM cards WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch card by ID")?;

    Ok(card)
}

/// Get multiple cards by IDs
pub async fn get_cards_by_ids(pool: &PgPool, ids: &[Uuid]) -> Result<Vec<Card>> {
    // PostgreSQL has a limit on the number of parameters
    // Chunk large ID lists to avoid hitting limits
    const CHUNK_SIZE: usize = 500; // Reduced chunk size for safety

    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut all_cards = Vec::with_capacity(ids.len());

    for chunk in ids.chunks(CHUNK_SIZE) {
        // Build a query with placeholders
        let placeholders: Vec<String> = (1..=chunk.len())
            .map(|i| format!("${}", i))
            .collect();
        let query_str = format!(
            "SELECT * FROM cards WHERE id IN ({})",
            placeholders.join(", ")
        );

        let mut query = sqlx::query_as::<_, Card>(&query_str);
        for id in chunk {
            query = query.bind(id);
        }

        let cards = query
            .fetch_all(pool)
            .await
            .context("Failed to fetch cards by IDs")?;

        all_cards.extend(cards);
    }

    Ok(all_cards)
}

/// Search cards by name (fuzzy match)
pub async fn search_cards_by_name(pool: &PgPool, name: &str, limit: i64) -> Result<Vec<Card>> {
    let cards = sqlx::query_as::<_, Card>(
        r#"
        SELECT * FROM cards
        WHERE to_tsvector('english', name) @@ plainto_tsquery('english', $1)
        ORDER BY name
        LIMIT $2
        "#
    )
    .bind(name)
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("Failed to search cards by name")?;

    Ok(cards)
}

/// Store query cache entry
pub async fn store_query_cache(
    pool: &PgPool,
    query_hash: &str,
    query_text: &str,
    result_ids: &[Uuid],
    total_cards: i32,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO query_cache (query_hash, query_text, result_ids, total_cards)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (query_hash) DO UPDATE SET
            result_ids = EXCLUDED.result_ids,
            total_cards = EXCLUDED.total_cards,
            last_accessed = NOW()
        "#,
    )
    .bind(query_hash)
    .bind(query_text)
    .bind(result_ids)
    .bind(total_cards)
    .execute(pool)
    .await
    .context("Failed to store query cache")?;

    Ok(())
}

/// Get query cache entry
pub async fn get_query_cache(pool: &PgPool, query_hash: &str) -> Result<Option<(Vec<Uuid>, i32)>> {
    let result: Option<(Vec<Uuid>, i32)> = sqlx::query_as(
        r#"
        UPDATE query_cache
        SET last_accessed = NOW()
        WHERE query_hash = $1
        RETURNING result_ids, total_cards
        "#,
    )
    .bind(query_hash)
    .fetch_optional(pool)
    .await
    .context("Failed to get query cache")?;

    Ok(result)
}

/// Record bulk data import
pub async fn record_bulk_import(
    pool: &PgPool,
    bulk_type: &str,
    download_uri: &str,
    updated_at: chrono::NaiveDateTime,
    total_cards: i32,
    file_size: i64,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO bulk_data_metadata (bulk_type, download_uri, updated_at, total_cards, file_size_bytes)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(bulk_type)
    .bind(download_uri)
    .bind(updated_at)
    .bind(total_cards)
    .bind(file_size)
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
