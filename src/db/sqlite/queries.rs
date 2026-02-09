use anyhow::{Context, Result};
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use crate::db::sqlite::connection::SqlitePool;
use crate::models::card::Card;

/// Insert a batch of cards into the database
pub fn insert_cards_batch(pool: &SqlitePool, cards: &[Card]) -> Result<()> {
    if cards.is_empty() {
        return Ok(());
    }

    let mut conn = pool.get().context("Failed to get connection from pool")?;
    let tx = conn.transaction().context("Failed to begin transaction")?;

    for card in cards {
        let colors_json = card.colors.as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());
        let color_identity_json = card.color_identity.as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());
        let keywords_json = card.keywords.as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());
        let prices_json = card.prices.as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());
        let image_uris_json = card.image_uris.as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());
        let card_faces_json = card.card_faces.as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());
        let legalities_json = card.legalities.as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());
        let raw_json = serde_json::to_string(&card.raw_json)
            .context("Failed to serialize raw_json")?;

        tx.execute(
            r#"
            INSERT INTO cards (
                id, oracle_id, name, mana_cost, cmc, type_line, oracle_text,
                colors, color_identity, set_code, set_name, collector_number,
                rarity, power, toughness, loyalty, keywords, prices, image_uris,
                card_faces, legalities, released_at, raw_json
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23
            )
            ON CONFLICT(id) DO UPDATE SET
                oracle_id = excluded.oracle_id,
                name = excluded.name,
                mana_cost = excluded.mana_cost,
                cmc = excluded.cmc,
                type_line = excluded.type_line,
                oracle_text = excluded.oracle_text,
                colors = excluded.colors,
                color_identity = excluded.color_identity,
                set_code = excluded.set_code,
                set_name = excluded.set_name,
                collector_number = excluded.collector_number,
                rarity = excluded.rarity,
                power = excluded.power,
                toughness = excluded.toughness,
                loyalty = excluded.loyalty,
                keywords = excluded.keywords,
                prices = excluded.prices,
                image_uris = excluded.image_uris,
                card_faces = excluded.card_faces,
                legalities = excluded.legalities,
                released_at = excluded.released_at,
                raw_json = excluded.raw_json,
                updated_at = CURRENT_TIMESTAMP
            "#,
            params![
                card.id.to_string(),
                card.oracle_id.map(|u| u.to_string()),
                &card.name,
                &card.mana_cost,
                card.cmc,
                &card.type_line,
                &card.oracle_text,
                colors_json,
                color_identity_json,
                &card.set_code,
                &card.set_name,
                &card.collector_number,
                &card.rarity,
                &card.power,
                &card.toughness,
                &card.loyalty,
                keywords_json,
                prices_json,
                image_uris_json,
                card_faces_json,
                legalities_json,
                card.released_at.map(|d| d.to_string()),
                raw_json,
            ],
        ).context("Failed to insert card")?;
    }

    tx.commit().context("Failed to commit transaction")?;
    Ok(())
}

/// Get a card by ID
pub fn get_card_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Card>> {
    let conn = pool.get().context("Failed to get connection from pool")?;
    
    let card = conn.query_row(
        "SELECT * FROM cards WHERE id = ?1",
        params![id.to_string()],
        row_to_card,
    ).optional().context("Failed to fetch card by ID")?;

    Ok(card)
}

/// Get multiple cards by IDs
pub fn get_cards_by_ids(pool: &SqlitePool, ids: &[Uuid]) -> Result<Vec<Card>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let conn = pool.get().context("Failed to get connection from pool")?;
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!("SELECT * FROM cards WHERE id IN ({})", placeholders);
    
    let mut stmt = conn.prepare(&query).context("Failed to prepare statement")?;
    let id_strings: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
    let params: Vec<&dyn rusqlite::ToSql> = id_strings.iter()
        .map(|s| s as &dyn rusqlite::ToSql)
        .collect();
    
    let cards = stmt.query_map(params.as_slice(), row_to_card)
        .context("Failed to query cards")?
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to map rows to cards")?;

    Ok(cards)
}

/// Search cards by name (fuzzy search)
pub fn search_cards_by_name(pool: &SqlitePool, name: &str, limit: i64) -> Result<Vec<Card>> {
    let conn = pool.get().context("Failed to get connection from pool")?;
    let search_pattern = format!("%{}%", name);
    
    let mut stmt = conn.prepare(
        "SELECT * FROM cards WHERE name LIKE ?1 COLLATE NOCASE LIMIT ?2"
    ).context("Failed to prepare statement")?;
    
    let cards = stmt.query_map(params![search_pattern, limit], row_to_card)
        .context("Failed to query cards")?
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to map rows to cards")?;

    Ok(cards)
}

/// Autocomplete card names by prefix (case-insensitive)
/// Returns distinct card names that start with the given prefix, sorted alphabetically
pub fn autocomplete_card_names(pool: &SqlitePool, prefix: &str, limit: i64) -> Result<Vec<String>> {
    let conn = pool.get().context("Failed to get connection from pool")?;
    let search_pattern = format!("{}%", prefix);
    
    let mut stmt = conn.prepare(
        "SELECT DISTINCT name FROM cards WHERE name LIKE ?1 COLLATE NOCASE ORDER BY name LIMIT ?2"
    ).context("Failed to prepare statement")?;
    
    let names = stmt.query_map(params![search_pattern, limit], |row| {
        row.get::<_, String>(0)
    })
        .context("Failed to query card names")?
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to map rows to names")?;

    Ok(names)
}

/// Store a query result in the cache
pub fn store_query_cache(
    pool: &SqlitePool,
    query_hash: &str,
    card_ids: &[Uuid],
    ttl_hours: i32,
) -> Result<()> {
    let conn = pool.get().context("Failed to get connection from pool")?;
    let card_ids_json = serde_json::to_string(card_ids)
        .context("Failed to serialize card IDs")?;

    conn.execute(
        r#"
        INSERT INTO query_cache (query_hash, card_ids, ttl_hours, expires_at)
        VALUES (?1, ?2, ?3, datetime('now', '+' || ?3 || ' hours'))
        ON CONFLICT(query_hash) DO UPDATE SET
            card_ids = excluded.card_ids,
            ttl_hours = excluded.ttl_hours,
            created_at = CURRENT_TIMESTAMP,
            expires_at = datetime('now', '+' || excluded.ttl_hours || ' hours')
        "#,
        params![query_hash, card_ids_json, ttl_hours],
    ).context("Failed to store query cache")?;

    Ok(())
}

/// Get cached query results
pub fn get_query_cache(pool: &SqlitePool, query_hash: &str) -> Result<Option<(Vec<Uuid>, i32)>> {
    let conn = pool.get().context("Failed to get connection from pool")?;
    
    let result = conn.query_row(
        r#"
        SELECT card_ids, ttl_hours FROM query_cache
        WHERE query_hash = ?1 AND expires_at > datetime('now')
        "#,
        params![query_hash],
        |row| {
            let card_ids_json: String = row.get(0)?;
            let ttl_hours: i32 = row.get(1)?;
            Ok((card_ids_json, ttl_hours))
        },
    ).optional().context("Failed to fetch query cache")?;

    if let Some((card_ids_json, ttl_hours)) = result {
        let card_ids: Vec<Uuid> = serde_json::from_str(&card_ids_json)
            .context("Failed to deserialize card IDs")?;
        Ok(Some((card_ids, ttl_hours)))
    } else {
        Ok(None)
    }
}

/// Record a bulk import operation
pub fn record_bulk_import(pool: &SqlitePool, total_cards: i32, source: &str) -> Result<()> {
    let conn = pool.get().context("Failed to get connection from pool")?;
    
    conn.execute(
        "INSERT INTO bulk_imports (total_cards, source) VALUES (?1, ?2)",
        params![total_cards, source],
    ).context("Failed to record bulk import")?;

    Ok(())
}

/// Clean old cache entries
pub fn clean_old_cache_entries(pool: &SqlitePool, hours: i32) -> Result<u64> {
    let conn = pool.get().context("Failed to get connection from pool")?;
    
    let deleted = conn.execute(
        "DELETE FROM query_cache WHERE expires_at < datetime('now', '-' || ?1 || ' hours')",
        params![hours],
    ).context("Failed to clean old cache entries")?;

    Ok(deleted as u64)
}

/// Helper function to convert a SQLite row to a Card
fn row_to_card(row: &rusqlite::Row) -> rusqlite::Result<Card> {
    let id_str: String = row.get("id")?;
    let id = Uuid::parse_str(&id_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })?;

    let oracle_id_str: Option<String> = row.get("oracle_id")?;
    let oracle_id = oracle_id_str.and_then(|s| Uuid::parse_str(&s).ok());

    let colors_json: Option<String> = row.get("colors")?;
    let colors = colors_json.and_then(|s| serde_json::from_str(&s).ok());

    let color_identity_json: Option<String> = row.get("color_identity")?;
    let color_identity = color_identity_json.and_then(|s| serde_json::from_str(&s).ok());

    let keywords_json: Option<String> = row.get("keywords")?;
    let keywords = keywords_json.and_then(|s| serde_json::from_str(&s).ok());

    let prices_json: Option<String> = row.get("prices")?;
    let prices = prices_json.and_then(|s| serde_json::from_str(&s).ok());

    let image_uris_json: Option<String> = row.get("image_uris")?;
    let image_uris = image_uris_json.and_then(|s| serde_json::from_str(&s).ok());

    let card_faces_json: Option<String> = row.get("card_faces")?;
    let card_faces = card_faces_json.and_then(|s| serde_json::from_str(&s).ok());

    let legalities_json: Option<String> = row.get("legalities")?;
    let legalities = legalities_json.and_then(|s| serde_json::from_str(&s).ok());

    let raw_json_str: String = row.get("raw_json")?;
    let raw_json = serde_json::from_str(&raw_json_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })?;

    let released_at_str: Option<String> = row.get("released_at")?;
    let released_at = released_at_str.and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());

    let created_at_str: Option<String> = row.get("created_at")?;
    let created_at = created_at_str.and_then(|s| chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").ok());

    let updated_at_str: Option<String> = row.get("updated_at")?;
    let updated_at = updated_at_str.and_then(|s| chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").ok());

    Ok(Card {
        id,
        oracle_id,
        name: row.get("name")?,
        mana_cost: row.get("mana_cost")?,
        cmc: row.get("cmc")?,
        type_line: row.get("type_line")?,
        oracle_text: row.get("oracle_text")?,
        colors,
        color_identity,
        set_code: row.get("set_code")?,
        set_name: row.get("set_name")?,
        collector_number: row.get("collector_number")?,
        rarity: row.get("rarity")?,
        power: row.get("power")?,
        toughness: row.get("toughness")?,
        loyalty: row.get("loyalty")?,
        keywords,
        prices,
        image_uris,
        card_faces,
        legalities,
        released_at,
        raw_json,
        created_at,
        updated_at,
    })
}

/// Execute a raw SQL query and return Card results
/// Note: SQLite has limited support for complex Scryfall queries
/// This is a basic implementation that may not handle all query types
pub fn execute_raw_query(pool: &SqlitePool, sql: &str, params: &[String]) -> Result<Vec<Card>> {
    let conn = pool.get().context("Failed to get connection from pool")?;
    
    // SQLite doesn't support the same query syntax as PostgreSQL
    // This is a simplified implementation
    let mut stmt = conn
        .prepare(sql)
        .context("Failed to prepare SQL statement")?;

    let cards = stmt
        .query_map(rusqlite::params_from_iter(params.iter()), |row| {
            row_to_card(row)
        })
        .context("Failed to execute query")?
        .collect::<Result<Vec<Card>, _>>()
        .context("Failed to parse query results")?;

    Ok(cards)
}

/// Execute a COUNT query and return the result
pub fn count_query(pool: &SqlitePool, sql: &str, params: &[String]) -> Result<usize> {
    let conn = pool.get().context("Failed to get connection from pool")?;
    
    let mut stmt = conn
        .prepare(sql)
        .context("Failed to prepare COUNT statement")?;

    let count: i64 = stmt
        .query_row(rusqlite::params_from_iter(params.iter()), |row| row.get(0))
        .context("Failed to execute COUNT query")?;

    Ok(count as usize)
}

/// Check if bulk data is loaded
pub fn check_bulk_data_loaded(pool: &SqlitePool) -> Result<bool> {
    let conn = pool.get().context("Failed to get connection from pool")?;
    
    let result: i64 = conn
        .query_row("SELECT COUNT(*) FROM cards", [], |row| row.get(0))
        .context("Failed to check if bulk data is loaded")?;

    Ok(result > 0)
}

/// Get the timestamp of the last bulk import
pub fn get_last_bulk_import(pool: &SqlitePool) -> Result<Option<chrono::NaiveDateTime>> {
    let conn = pool.get().context("Failed to get connection from pool")?;
    
    let result: Option<String> = conn
        .query_row(
            "SELECT imported_at FROM bulk_data_metadata ORDER BY imported_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .context("Failed to get last bulk import timestamp")?;

    if let Some(timestamp_str) = result {
        let dt = chrono::NaiveDateTime::parse_from_str(&timestamp_str, "%Y-%m-%d %H:%M:%S")
            .context("Failed to parse timestamp")?;
        Ok(Some(dt))
    } else {
        Ok(None)
    }
}

/// Get the total count of cards in the database
pub fn get_card_count(pool: &SqlitePool) -> Result<i64> {
    let conn = pool.get().context("Failed to get connection from pool")?;
    
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM cards", [], |row| row.get(0))
        .context("Failed to get card count")?;

    Ok(count)
}

/// Get the total count of query cache entries
pub fn get_cache_entry_count(pool: &SqlitePool) -> Result<i64> {
    let conn = pool.get().context("Failed to get connection from pool")?;
    
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM query_cache", [], |row| row.get(0))
        .context("Failed to get cache entry count")?;

    Ok(count)
}
