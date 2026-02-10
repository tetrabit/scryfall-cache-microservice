#[cfg(feature = "postgres")]
use anyhow::{Context, Result};
#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "postgres")]
use tracing::info;

#[cfg(feature = "postgres")]
const MIGRATION_SQL: &str = concat!(
    include_str!("../../migrations/001_initial_schema.sql"),
    "\n",
    include_str!("../../migrations/002_fix_cmc_type.sql"),
    "\n",
    include_str!("../../migrations/003_add_performance_indexes.sql"),
);

#[cfg(feature = "postgres")]
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    info!("Running database migrations...");

    // Split the migration SQL into individual statements
    // We need to handle this carefully because of function definitions with semicolons
    let statements = split_sql_statements(MIGRATION_SQL);

    for (i, statement) in statements.iter().enumerate() {
        let trimmed = statement.trim();
        if trimmed.is_empty() || trimmed.starts_with("--") {
            continue;
        }

        sqlx::query(trimmed)
            .execute(pool)
            .await
            .with_context(|| format!("Failed to execute migration statement {}: {}", i + 1, &trimmed[..trimmed.len().min(100)]))?;
    }

    info!("Database migrations completed successfully");
    Ok(())
}

#[cfg(feature = "postgres")]
fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_function = false;
    let mut dollar_quote_tag: Option<String> = None;

    for line in sql.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with("--") {
            continue;
        }

        // Check for function start
        if trimmed.contains("CREATE OR REPLACE FUNCTION") || trimmed.contains("CREATE FUNCTION") {
            in_function = true;
        }

        // Check for dollar quotes ($$)
        if let Some(pos) = trimmed.find("$$") {
            if dollar_quote_tag.is_none() {
                // Extract the tag between $ symbols if any (e.g., $body$)
                let tag = if pos > 0 && trimmed[..pos].ends_with('$') {
                    "$$".to_string()
                } else {
                    "$$".to_string()
                };
                dollar_quote_tag = Some(tag);
            } else {
                dollar_quote_tag = None;
            }
        }

        current.push_str(line);
        current.push('\n');

        // End statement on semicolon if we're not in a function or dollar quote
        if trimmed.ends_with(';') && !in_function && dollar_quote_tag.is_none() {
            statements.push(current.trim().to_string());
            current.clear();
        }

        // Check for function end
        if in_function && trimmed.contains("language") && trimmed.ends_with(';') {
            in_function = false;
            statements.push(current.trim().to_string());
            current.clear();
        }
    }

    // Add any remaining content
    if !current.trim().is_empty() {
        statements.push(current.trim().to_string());
    }

    statements
}

#[cfg(feature = "postgres")]
pub async fn check_bulk_data_loaded(pool: &PgPool) -> Result<bool> {
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cards")
        .fetch_one(pool)
        .await
        .context("Failed to check if bulk data is loaded")?;

    Ok(result.0 > 0)
}

#[cfg(feature = "postgres")]
pub async fn get_card_count(pool: &PgPool) -> Result<i64> {
    let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cards")
        .fetch_one(pool)
        .await
        .context("Failed to get card count")?;

    Ok(result.0)
}

#[cfg(feature = "postgres")]
pub async fn get_last_bulk_import(pool: &PgPool) -> Result<Option<chrono::NaiveDateTime>> {
    let result: Option<(chrono::NaiveDateTime,)> = sqlx::query_as(
        "SELECT imported_at FROM bulk_data_metadata ORDER BY imported_at DESC LIMIT 1"
    )
    .fetch_optional(pool)
    .await
    .context("Failed to get last bulk import timestamp")?;

    Ok(result.map(|r| r.0))
}

#[cfg(all(test, feature = "postgres"))]
mod tests {
    use super::MIGRATION_SQL;

    #[test]
    fn migration_sql_includes_phase_2_indexes() {
        let sql = MIGRATION_SQL;
        assert!(
            sql.contains("idx_cards_colors_type"),
            "Missing composite colors/type index"
        );
        assert!(
            sql.contains("idx_cards_cmc_colors"),
            "Missing cmc/colors index"
        );
        assert!(
            sql.contains("idx_cards_set_rarity"),
            "Missing set/rarity index"
        );
        assert!(
            sql.contains("idx_cards_set_collector"),
            "Missing set/collector index"
        );
    }

    #[test]
    fn migration_sql_includes_cmc_type_fix() {
        let sql = MIGRATION_SQL;
        assert!(
            sql.contains("ALTER TABLE cards ALTER COLUMN cmc TYPE DOUBLE PRECISION"),
            "Missing CMC type fix migration"
        );
    }
}
