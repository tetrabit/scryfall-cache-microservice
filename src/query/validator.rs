use anyhow::{anyhow, Result};
use std::collections::HashSet;

use super::limits::QueryLimits;
use super::parser::{Operator, QueryNode};

/// Valid field names for card queries
const VALID_FIELDS: &[&str] = &[
    "name",
    "type",
    "oracle",
    "color",
    "colors",
    "cmc",
    "mana",
    "power",
    "toughness",
    "set",
    "rarity",
    "artist",
    "flavor",
    "border",
    "frame",
    "layout",
    "loyalty",
];

/// Fields that support numeric operators (>, <, >=, <=)
const NUMERIC_FIELDS: &[&str] = &["cmc", "power", "toughness", "loyalty"];

/// Valid color codes
const VALID_COLORS: &[char] = &['w', 'u', 'b', 'r', 'g', 'c'];

pub struct QueryValidator {
    limits: QueryLimits,
    valid_fields: HashSet<String>,
    numeric_fields: HashSet<String>,
}

impl QueryValidator {
    pub fn new(limits: QueryLimits) -> Self {
        Self {
            limits,
            valid_fields: VALID_FIELDS.iter().map(|s| s.to_string()).collect(),
            numeric_fields: NUMERIC_FIELDS.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Get reference to query limits
    pub fn limits(&self) -> &QueryLimits {
        &self.limits
    }

    /// Validate query string before parsing
    pub fn validate_query_string(&self, query: &str) -> Result<()> {
        // Check length
        if query.len() > self.limits.max_query_length {
            return Err(anyhow!(
                "Query too long: maximum {} characters allowed, got {}",
                self.limits.max_query_length,
                query.len()
            ));
        }

        // Check for balanced parentheses
        let mut paren_count = 0;
        for ch in query.chars() {
            match ch {
                '(' => paren_count += 1,
                ')' => {
                    paren_count -= 1;
                    if paren_count < 0 {
                        return Err(anyhow!(
                            "Unbalanced parentheses: too many closing parentheses"
                        ));
                    }
                }
                _ => {}
            }
        }
        if paren_count != 0 {
            return Err(anyhow!(
                "Unbalanced parentheses: {} unclosed parentheses",
                paren_count
            ));
        }

        Ok(())
    }

    /// Validate parsed query AST
    pub fn validate_ast(&self, node: &QueryNode) -> Result<()> {
        // Check nesting depth
        let depth = self.calculate_depth(node);
        if depth > self.limits.max_nesting_depth {
            return Err(anyhow!(
                "Query too complex: maximum nesting depth is {}, got {}",
                self.limits.max_nesting_depth,
                depth
            ));
        }

        // Check OR clause count
        let or_count = self.count_or_clauses(node);
        if or_count > self.limits.max_or_clauses {
            return Err(anyhow!(
                "Query too complex: maximum {} OR clauses allowed, got {}",
                self.limits.max_or_clauses,
                or_count
            ));
        }

        // Validate filters
        self.validate_node(node)?;

        Ok(())
    }

    /// Recursively validate query node
    fn validate_node(&self, node: &QueryNode) -> Result<()> {
        match node {
            QueryNode::And(children) | QueryNode::Or(children) => {
                for child in children {
                    self.validate_node(child)?;
                }
            }
            QueryNode::Not(child) => {
                self.validate_node(child)?;
            }
            QueryNode::Filter(filter) => {
                self.validate_filter(filter)?;
            }
        }
        Ok(())
    }

    /// Validate a single filter
    fn validate_filter(&self, filter: &super::parser::Filter) -> Result<()> {
        let field = filter.field.to_lowercase();

        // Check if field name is valid
        if !self.valid_fields.contains(&field) {
            return Err(anyhow!(
                "Invalid field name '{}': expected one of [{}]",
                filter.field,
                VALID_FIELDS.join(", ")
            ));
        }

        // Check if operator is valid for this field
        if !self.numeric_fields.contains(&field) {
            match filter.operator {
                Operator::GreaterThan
                | Operator::LessThan
                | Operator::GreaterThanOrEqual
                | Operator::LessThanOrEqual => {
                    return Err(anyhow!(
                        "Operator '{}' not valid for text field '{}'. Numeric operators (>, <, >=, <=) only work with: {}",
                        filter.operator,
                        filter.field,
                        NUMERIC_FIELDS.join(", ")
                    ));
                }
                _ => {}
            }
        }

        // Validate color codes
        if field == "color" || field == "colors" {
            for ch in filter.value.chars() {
                if !VALID_COLORS.contains(&ch.to_lowercase().next().unwrap_or(' ')) {
                    return Err(anyhow!(
                        "Invalid color code '{}' in value '{}': valid colors are {}",
                        ch,
                        filter.value,
                        VALID_COLORS.iter().collect::<String>()
                    ));
                }
            }
        }

        Ok(())
    }

    /// Calculate maximum nesting depth
    fn calculate_depth(&self, node: &QueryNode) -> usize {
        match node {
            QueryNode::And(children) | QueryNode::Or(children) => {
                1 + children
                    .iter()
                    .map(|child| self.calculate_depth(child))
                    .max()
                    .unwrap_or(0)
            }
            QueryNode::Not(child) => 1 + self.calculate_depth(child),
            QueryNode::Filter(_) => 1,
        }
    }

    /// Count OR clauses
    fn count_or_clauses(&self, node: &QueryNode) -> usize {
        match node {
            QueryNode::Or(children) => {
                1 + children
                    .iter()
                    .map(|child| self.count_or_clauses(child))
                    .sum::<usize>()
            }
            QueryNode::And(children) => children
                .iter()
                .map(|child| self.count_or_clauses(child))
                .sum(),
            QueryNode::Not(child) => self.count_or_clauses(child),
            QueryNode::Filter(_) => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_too_long() {
        let validator = QueryValidator::new(QueryLimits {
            max_query_length: 10,
            ..Default::default()
        });
        let result = validator.validate_query_string("this is a very long query");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Query too long"));
    }

    #[test]
    fn test_unbalanced_parentheses() {
        let validator = QueryValidator::new(QueryLimits::default());
        assert!(validator.validate_query_string("(name:sol").is_err());
        assert!(validator.validate_query_string("name:sol)").is_err());
        assert!(validator.validate_query_string("(name:sol)").is_ok());
    }

    #[test]
    fn test_invalid_field_name() {
        let validator = QueryValidator::new(QueryLimits::default());
        let filter = super::super::parser::Filter {
            field: "invalid_field".to_string(),
            operator: Operator::Equal,
            value: "test".to_string(),
        };
        let result = validator.validate_filter(&filter);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid field name"));
    }

    #[test]
    fn test_numeric_operator_on_text_field() {
        let validator = QueryValidator::new(QueryLimits::default());
        let filter = super::super::parser::Filter {
            field: "name".to_string(),
            operator: Operator::GreaterThan,
            value: "5".to_string(),
        };
        let result = validator.validate_filter(&filter);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not valid for text field"));
    }

    #[test]
    fn test_valid_filter() {
        let validator = QueryValidator::new(QueryLimits::default());
        let filter = super::super::parser::Filter {
            field: "name".to_string(),
            operator: Operator::Contains,
            value: "lightning".to_string(),
        };
        assert!(validator.validate_filter(&filter).is_ok());
    }
}
