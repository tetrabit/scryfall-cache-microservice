use anyhow::{Context, Result};
use sqlx::PgPool;
use tracing::debug;

use crate::models::card::Card;
use crate::query::parser::{Filter, Operator, QueryNode, QueryParser};

pub struct QueryExecutor {
    pool: PgPool,
}

impl QueryExecutor {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Execute a Scryfall query and return matching cards
    pub async fn execute(&self, query: &str, limit: Option<i64>) -> Result<Vec<Card>> {
        debug!("Executing query: {}", query);

        // Parse the query
        let ast = QueryParser::parse(query)
            .context("Failed to parse query")?;

        // Build SQL WHERE clause
        let (where_clause, params) = self.build_where_clause(&ast)?;

        // Build and execute SQL query
        let sql = format!(
            "SELECT * FROM cards WHERE {} ORDER BY name LIMIT ${}",
            where_clause,
            params.len() + 1
        );

        debug!("Generated SQL: {}", sql);

        let limit = limit.unwrap_or(100);
        let mut query_builder = sqlx::query_as::<_, Card>(&sql);

        // Bind all parameters
        for param in params {
            query_builder = query_builder.bind(param);
        }
        query_builder = query_builder.bind(limit);

        let cards = query_builder
            .fetch_all(&self.pool)
            .await
            .context("Failed to execute query")?;

        debug!("Query returned {} cards", cards.len());
        Ok(cards)
    }

    /// Build WHERE clause from AST
    fn build_where_clause(&self, node: &QueryNode) -> Result<(String, Vec<String>)> {
        let mut params = Vec::new();
        let clause = self.build_where_clause_inner(node, &mut params)?;
        Ok((clause, params))
    }

    fn build_where_clause_inner(&self, node: &QueryNode, params: &mut Vec<String>) -> Result<String> {
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
            "color" | "c" => {
                self.build_color_clause(&filter.value, &filter.operator, params)
            }
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
                Ok(self.build_numeric_comparison("toughness::numeric", param_index, &filter.operator))
            }
            "loyalty" | "loy" => {
                params.push(filter.value.clone());
                Ok(self.build_numeric_comparison("loyalty::numeric", param_index, &filter.operator))
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

    fn build_numeric_comparison(&self, field: &str, param_index: usize, operator: &Operator) -> String {
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

    fn build_color_clause(&self, value: &str, operator: &Operator, params: &mut Vec<String>) -> Result<String> {
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
            Operator::Equal | Operator::Contains => {
                Ok(format!("${} = ANY(colors)", param_index))
            }
            Operator::NotEqual => {
                Ok(format!("NOT (${} = ANY(colors))", param_index))
            }
            _ => Ok(format!("${} = ANY(colors)", param_index)),
        }
    }

    fn build_color_identity_clause(&self, value: &str, operator: &Operator, params: &mut Vec<String>) -> Result<String> {
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
            Operator::NotEqual => {
                Ok(format!("NOT (${} = ANY(color_identity))", param_index))
            }
            _ => Ok(format!("${} = ANY(color_identity)", param_index)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_filter_clause() {
        let filter = Filter {
            field: "name".to_string(),
            operator: Operator::Contains,
            value: "lightning".to_string(),
        };

        let mut params = Vec::new();
        let clause = QueryExecutor::new(
            sqlx::PgPool::connect("postgresql://localhost/test")
                .await
                .unwrap()
        )
        .build_filter_clause(&filter, &mut params)
        .unwrap();

        assert!(clause.contains("to_tsvector"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "lightning");
    }
}
