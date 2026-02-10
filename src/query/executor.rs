use anyhow::{Context, Result};
use tracing::debug;

use crate::db::Database;
use crate::models::card::Card;
use crate::query::parser::{Filter, Operator, QueryNode, QueryParser};

pub struct QueryExecutor {
    db: Database,
}

impl QueryExecutor {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Execute a Scryfall query and return matching cards
    pub async fn execute(&self, query: &str, limit: Option<i64>) -> Result<Vec<Card>> {
        debug!("Executing query: {}", query);

        // Parse the query
        let ast = QueryParser::parse(query).context("Failed to parse query")?;

        // Build SQL WHERE clause
        let (where_clause, params) = self.build_where_clause(&ast)?;

        // Build SQL query with optional LIMIT clause
        let (sql, final_params) = if let Some(limit_val) = limit {
            let sql = format!(
                "SELECT * FROM cards WHERE {} ORDER BY name LIMIT ${}",
                where_clause,
                params.len() + 1
            );
            let mut params_with_limit = params;
            params_with_limit.push(limit_val.to_string());
            (sql, params_with_limit)
        } else {
            // No limit - return all matching cards
            let sql = format!("SELECT * FROM cards WHERE {} ORDER BY name", where_clause);
            (sql, params)
        };

        debug!("Generated SQL: {}", sql);

        let cards = self
            .db
            .execute_raw_query(&sql, &final_params)
            .await
            .map_err(|e| {
                tracing::error!("Database query failed: {:?}", e);
                anyhow::anyhow!("Failed to execute query: {}", e)
            })?;

        debug!("Query returned {} cards", cards.len());
        Ok(cards)
    }

    /// Count total number of matching cards without fetching them
    pub async fn count_matches(&self, query: &str) -> Result<usize> {
        debug!("Counting matches for query: {}", query);

        // Parse the query
        let ast = QueryParser::parse(query).context("Failed to parse query")?;

        // Build SQL WHERE clause
        let (where_clause, params) = self.build_where_clause(&ast)?;

        // Build COUNT query
        let sql = format!("SELECT COUNT(*) FROM cards WHERE {}", where_clause);

        debug!("Generated COUNT SQL: {}", sql);

        // Execute count query
        let count = self.db.count_query(&sql, &params).await.map_err(|e| {
            tracing::error!("Count query failed: {:?}", e);
            anyhow::anyhow!("Failed to count matches: {}", e)
        })?;

        debug!("Query matched {} cards", count);
        Ok(count)
    }

    /// Execute a paginated query, returning only the requested page of results
    pub async fn execute_paginated(
        &self,
        query: &str,
        page: usize,
        page_size: usize,
    ) -> Result<(Vec<Card>, usize)> {
        debug!(
            "Executing paginated query: query='{}', page={}, page_size={}",
            query, page, page_size
        );

        // Parse the query
        let ast = QueryParser::parse(query).context("Failed to parse query")?;

        // Build SQL WHERE clause
        let (where_clause, params) = self.build_where_clause(&ast)?;

        // First, get total count (fast - no data transfer)
        let count_sql = format!("SELECT COUNT(*) FROM cards WHERE {}", where_clause);
        let total = self
            .db
            .count_query(&count_sql, &params)
            .await
            .context("Failed to count total matches")?;

        // Calculate offset
        let offset = (page.saturating_sub(1)) * page_size;

        // Build paginated query with LIMIT and OFFSET
        let sql = format!(
            "SELECT * FROM cards WHERE {} ORDER BY name LIMIT {} OFFSET {}",
            where_clause, page_size, offset
        );

        debug!("Generated paginated SQL: {}", sql);
        debug!(
            "Total matches: {}, fetching page {} ({} cards starting at offset {})",
            total, page, page_size, offset
        );

        // Execute query for only the requested page
        let cards = self
            .db
            .execute_raw_query(&sql, &params)
            .await
            .map_err(|e| {
                tracing::error!("Paginated query failed: {:?}", e);
                anyhow::anyhow!("Failed to execute paginated query: {}", e)
            })?;

        debug!(
            "Query returned {} cards (page {} of {})",
            cards.len(),
            page,
            total.div_ceil(page_size)
        );

        Ok((cards, total))
    }

    /// Build WHERE clause from AST
    fn build_where_clause(&self, node: &QueryNode) -> Result<(String, Vec<String>)> {
        let mut params = Vec::new();
        let clause = self.build_where_clause_inner(node, &mut params)?;
        Ok((clause, params))
    }

    fn build_where_clause_inner(
        &self,
        node: &QueryNode,
        params: &mut Vec<String>,
    ) -> Result<String> {
        match node {
            QueryNode::And(nodes) => {
                let clauses: Result<Vec<String>> = nodes
                    .iter()
                    .map(|n| self.build_where_clause_inner(n, params))
                    .collect();
                Ok(format!("({})", clauses?.join(" AND ")))
            }
            QueryNode::Or(nodes) => {
                let clauses: Result<Vec<String>> = nodes
                    .iter()
                    .map(|n| self.build_where_clause_inner(n, params))
                    .collect();
                Ok(format!("({})", clauses?.join(" OR ")))
            }
            QueryNode::Not(inner) => {
                let clause = self.build_where_clause_inner(inner, params)?;
                Ok(format!("NOT ({})", clause))
            }
            QueryNode::Filter(filter) => self.build_filter_clause(filter, params),
        }
    }

    fn build_filter_clause(&self, filter: &Filter, params: &mut Vec<String>) -> Result<String> {
        let param_index = params.len() + 1;

        match filter.field.as_str() {
            "name" => {
                params.push(filter.value.clone());
                Ok(self.build_text_search("name", param_index, &filter.operator))
            }
            "oracle" | "oracle_text" => {
                params.push(filter.value.clone());
                Ok(self.build_text_search("oracle_text", param_index, &filter.operator))
            }
            "type" | "type_line" => {
                params.push(filter.value.clone());
                Ok(self.build_text_search("type_line", param_index, &filter.operator))
            }
            "color" | "c" => self.build_color_clause(&filter.value, &filter.operator, params),
            "color_identity" | "id" | "identity" => {
                self.build_color_identity_clause(&filter.value, &filter.operator, params)
            }
            "set" | "s" => {
                params.push(filter.value.to_lowercase());
                Ok(format!("set_code = ${}", param_index))
            }
            "rarity" | "r" => {
                params.push(filter.value.to_lowercase());
                Ok(format!("rarity = ${}", param_index))
            }
            "cmc" => {
                params.push(filter.value.clone());
                Ok(self.build_numeric_comparison("cmc", param_index, &filter.operator))
            }
            "power" | "pow" => {
                params.push(filter.value.clone());
                Ok(self.build_numeric_comparison("power::numeric", param_index, &filter.operator))
            }
            "toughness" | "tou" => {
                params.push(filter.value.clone());
                Ok(self.build_numeric_comparison(
                    "toughness::numeric",
                    param_index,
                    &filter.operator,
                ))
            }
            "loyalty" | "loy" => {
                params.push(filter.value.clone());
                Ok(
                    self.build_numeric_comparison(
                        "loyalty::numeric",
                        param_index,
                        &filter.operator,
                    ),
                )
            }
            _ => {
                // Default: treat as name search
                params.push(filter.value.clone());
                Ok(self.build_text_search("name", param_index, &filter.operator))
            }
        }
    }

    fn build_text_search(&self, field: &str, param_index: usize, operator: &Operator) -> String {
        match operator {
            Operator::Equal => format!("LOWER({}) = LOWER(${})", field, param_index),
            Operator::Contains => format!(
                "to_tsvector('english', {}) @@ plainto_tsquery('english', ${})",
                field, param_index
            ),
            Operator::Regex => format!("{} ~ ${}", field, param_index),
            _ => format!("{} ILIKE '%' || ${} || '%'", field, param_index),
        }
    }

    fn build_numeric_comparison(
        &self,
        field: &str,
        param_index: usize,
        operator: &Operator,
    ) -> String {
        let op = match operator {
            Operator::Equal | Operator::Contains => "=",
            Operator::NotEqual => "!=",
            Operator::GreaterThan => ">",
            Operator::LessThan => "<",
            Operator::GreaterThanOrEqual => ">=",
            Operator::LessThanOrEqual => "<=",
            Operator::Regex => "=",
        };

        format!("{} {} ${}::numeric", field, op, param_index)
    }

    fn build_color_clause(
        &self,
        value: &str,
        operator: &Operator,
        params: &mut Vec<String>,
    ) -> Result<String> {
        let param_index = params.len() + 1;

        // Parse color codes (w, u, b, r, g, c for colorless)
        let colors: Vec<String> = value
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| {
                match c.to_lowercase().next().unwrap() {
                    'w' => "W",
                    'u' => "U",
                    'b' => "B",
                    'r' => "R",
                    'g' => "G",
                    'c' => "C",
                    _ => "",
                }
                .to_string()
            })
            .filter(|s| !s.is_empty())
            .collect();

        if colors.is_empty() {
            return Ok("colors IS NULL OR colors = '{}'".to_string());
        }

        params.push(colors[0].clone());

        match operator {
            Operator::Equal | Operator::Contains => Ok(format!("${} = ANY(colors)", param_index)),
            Operator::NotEqual => Ok(format!("NOT (${} = ANY(colors))", param_index)),
            _ => Ok(format!("${} = ANY(colors)", param_index)),
        }
    }

    fn build_color_identity_clause(
        &self,
        value: &str,
        operator: &Operator,
        params: &mut Vec<String>,
    ) -> Result<String> {
        let param_index = params.len() + 1;

        // Parse color codes
        let colors: Vec<String> = value
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| {
                match c.to_lowercase().next().unwrap() {
                    'w' => "W",
                    'u' => "U",
                    'b' => "B",
                    'r' => "R",
                    'g' => "G",
                    'c' => "C",
                    _ => "",
                }
                .to_string()
            })
            .filter(|s| !s.is_empty())
            .collect();

        if colors.is_empty() {
            return Ok("color_identity IS NULL OR color_identity = '{}'".to_string());
        }

        params.push(colors[0].clone());

        match operator {
            Operator::Equal | Operator::Contains => {
                Ok(format!("${} = ANY(color_identity)", param_index))
            }
            Operator::NotEqual => Ok(format!("NOT (${} = ANY(color_identity))", param_index)),
            _ => Ok(format!("${} = ANY(color_identity)", param_index)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::any::Any;
    use uuid::Uuid;

    #[derive(Debug)]
    struct TestDb;

    #[async_trait]
    impl crate::db::DatabaseBackend for TestDb {
        async fn insert_cards_batch(&self, _cards: &[Card]) -> anyhow::Result<()> {
            anyhow::bail!("not implemented")
        }

        async fn get_card_by_id(&self, _id: Uuid) -> anyhow::Result<Option<Card>> {
            anyhow::bail!("not implemented")
        }

        async fn get_cards_by_ids(&self, _ids: &[Uuid]) -> anyhow::Result<Vec<Card>> {
            anyhow::bail!("not implemented")
        }

        async fn search_cards_by_name(
            &self,
            _name: &str,
            _limit: i64,
        ) -> anyhow::Result<Vec<Card>> {
            anyhow::bail!("not implemented")
        }

        async fn autocomplete_card_names(
            &self,
            _prefix: &str,
            _limit: i64,
        ) -> anyhow::Result<Vec<String>> {
            anyhow::bail!("not implemented")
        }

        async fn store_query_cache(
            &self,
            _query_hash: &str,
            _card_ids: &[Uuid],
            _ttl_hours: i32,
        ) -> anyhow::Result<()> {
            anyhow::bail!("not implemented")
        }

        async fn get_query_cache(
            &self,
            _query_hash: &str,
        ) -> anyhow::Result<Option<(Vec<Uuid>, i32)>> {
            anyhow::bail!("not implemented")
        }

        async fn record_bulk_import(&self, _total_cards: i32, _source: &str) -> anyhow::Result<()> {
            anyhow::bail!("not implemented")
        }

        async fn clean_old_cache_entries(&self, _hours: i32) -> anyhow::Result<u64> {
            anyhow::bail!("not implemented")
        }

        async fn test_connection(&self) -> anyhow::Result<()> {
            Ok(())
        }

        async fn execute_raw_query(
            &self,
            _sql: &str,
            _params: &[String],
        ) -> anyhow::Result<Vec<Card>> {
            anyhow::bail!("not implemented")
        }

        async fn count_query(&self, _sql: &str, _params: &[String]) -> anyhow::Result<usize> {
            anyhow::bail!("not implemented")
        }

        async fn check_bulk_data_loaded(&self) -> anyhow::Result<bool> {
            anyhow::bail!("not implemented")
        }

        async fn get_last_bulk_import(&self) -> anyhow::Result<Option<chrono::NaiveDateTime>> {
            anyhow::bail!("not implemented")
        }

        async fn get_card_count(&self) -> anyhow::Result<i64> {
            anyhow::bail!("not implemented")
        }

        async fn get_cache_entry_count(&self) -> anyhow::Result<i64> {
            anyhow::bail!("not implemented")
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn test_build_filter_clause() {
        let filter = Filter {
            field: "name".to_string(),
            operator: Operator::Contains,
            value: "lightning".to_string(),
        };

        // This test only checks the WHERE clause building logic,
        // which doesn't require a database connection
        let mock_db = std::sync::Arc::new(TestDb) as crate::db::Database;

        let executor = QueryExecutor::new(mock_db);
        let mut params = Vec::new();
        let clause = executor.build_filter_clause(&filter, &mut params).unwrap();

        assert!(clause.contains("to_tsvector"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "lightning");
    }
}
