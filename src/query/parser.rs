use anyhow::Result;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum QueryNode {
    And(Vec<QueryNode>),
    Or(Vec<QueryNode>),
    Not(Box<QueryNode>),
    Filter(Filter),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    pub field: String,
    pub operator: Operator,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Contains,
    Regex,
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operator::Equal => write!(f, "="),
            Operator::NotEqual => write!(f, "!="),
            Operator::GreaterThan => write!(f, ">"),
            Operator::LessThan => write!(f, "<"),
            Operator::GreaterThanOrEqual => write!(f, ">="),
            Operator::LessThanOrEqual => write!(f, "<="),
            Operator::Contains => write!(f, ":"),
            Operator::Regex => write!(f, "~"),
        }
    }
}

pub struct QueryParser {
    tokens: Vec<String>,
    position: usize,
}

impl QueryParser {
    pub fn new(query: &str) -> Self {
        let tokens = Self::tokenize(query);
        Self {
            tokens,
            position: 0,
        }
    }

    /// Parse a Scryfall query into an AST
    pub fn parse(query: &str) -> Result<QueryNode> {
        let mut parser = Self::new(query);
        parser.parse_expression()
    }

    /// Tokenize the query string
    fn tokenize(query: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let chars = query.chars().peekable();

        for ch in chars {
            match ch {
                '"' => {
                    in_quotes = !in_quotes;
                    current.push(ch);
                }
                ' ' if !in_quotes => {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                }
                '(' | ')' if !in_quotes => {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    tokens.push(ch.to_string());
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.is_empty() {
            tokens.push(current);
        }

        tokens
    }

    fn current(&self) -> Option<&String> {
        self.tokens.get(self.position)
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn parse_expression(&mut self) -> Result<QueryNode> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<QueryNode> {
        let mut left = self.parse_and()?;

        while let Some(token) = self.current() {
            if token.to_lowercase() == "or" {
                self.advance();
                let right = self.parse_and()?;

                left = match left {
                    QueryNode::Or(mut nodes) => {
                        nodes.push(right);
                        QueryNode::Or(nodes)
                    }
                    _ => QueryNode::Or(vec![left, right]),
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<QueryNode> {
        let mut terms = Vec::new();
        terms.push(self.parse_term()?);

        while let Some(token) = self.current() {
            if token == ")" || token.to_lowercase() == "or" {
                break;
            }

            // Skip explicit AND
            if token.to_lowercase() == "and" {
                self.advance();
            }

            terms.push(self.parse_term()?);
        }

        if terms.len() == 1 {
            Ok(terms.into_iter().next().unwrap())
        } else {
            Ok(QueryNode::And(terms))
        }
    }

    fn parse_term(&mut self) -> Result<QueryNode> {
        let token = self
            .current()
            .ok_or_else(|| anyhow::anyhow!("Unexpected end of query"))?
            .clone();

        if token == "(" {
            self.advance();
            let expr = self.parse_expression()?;
            if self.current() == Some(&")".to_string()) {
                self.advance();
            }
            return Ok(expr);
        }

        if token.to_lowercase() == "not" || token == "-" {
            self.advance();
            let term = self.parse_term()?;
            return Ok(QueryNode::Not(Box::new(term)));
        }

        self.parse_filter()
    }

    fn parse_filter(&mut self) -> Result<QueryNode> {
        let token = self
            .current()
            .ok_or_else(|| anyhow::anyhow!("Expected filter"))?
            .clone();

        self.advance();

        // Parse field:value or field>=value patterns
        if let Some((field, rest)) = token.split_once(':') {
            let (operator, value) = self.parse_operator_and_value(rest)?;

            Ok(QueryNode::Filter(Filter {
                field: self.normalize_field(field),
                operator,
                value: value.trim_matches('"').to_string(),
            }))
        } else {
            // Default to name search
            Ok(QueryNode::Filter(Filter {
                field: "name".to_string(),
                operator: Operator::Contains,
                value: token.trim_matches('"').to_string(),
            }))
        }
    }

    fn parse_operator_and_value(&self, s: &str) -> Result<(Operator, String)> {
        if let Some(rest) = s.strip_prefix(">=") {
            Ok((Operator::GreaterThanOrEqual, rest.to_string()))
        } else if let Some(rest) = s.strip_prefix("<=") {
            Ok((Operator::LessThanOrEqual, rest.to_string()))
        } else if let Some(rest) = s.strip_prefix('>') {
            Ok((Operator::GreaterThan, rest.to_string()))
        } else if let Some(rest) = s.strip_prefix('<') {
            Ok((Operator::LessThan, rest.to_string()))
        } else if let Some(rest) = s.strip_prefix("!=") {
            Ok((Operator::NotEqual, rest.to_string()))
        } else if let Some(rest) = s.strip_prefix('=') {
            Ok((Operator::Equal, rest.to_string()))
        } else if s.starts_with('/') && s.ends_with('/') && s.len() > 2 {
            Ok((Operator::Regex, s[1..s.len() - 1].to_string()))
        } else {
            Ok((Operator::Contains, s.to_string()))
        }
    }

    fn normalize_field(&self, field: &str) -> String {
        match field.to_lowercase().as_str() {
            "c" => "color",
            "id" | "identity" => "color_identity",
            "t" => "type",
            "o" => "oracle",
            "s" => "set",
            "r" => "rarity",
            "pow" => "power",
            "tou" => "toughness",
            "loy" => "loyalty",
            _ => field,
        }
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let tokens = QueryParser::tokenize("name:lightning c:red");
        assert_eq!(tokens, vec!["name:lightning", "c:red"]);
    }

    #[test]
    fn test_parse_simple_filter() {
        let ast = QueryParser::parse("name:lightning").unwrap();
        match ast {
            QueryNode::Filter(filter) => {
                assert_eq!(filter.field, "name");
                assert_eq!(filter.value, "lightning");
            }
            _ => panic!("Expected Filter node"),
        }
    }

    #[test]
    fn test_parse_and() {
        let ast = QueryParser::parse("c:red t:creature").unwrap();
        match ast {
            QueryNode::And(nodes) => {
                assert_eq!(nodes.len(), 2);
            }
            _ => panic!("Expected And node"),
        }
    }

    #[test]
    fn test_parse_or() {
        let ast = QueryParser::parse("c:red or c:blue").unwrap();
        match ast {
            QueryNode::Or(nodes) => {
                assert_eq!(nodes.len(), 2);
            }
            _ => panic!("Expected Or node"),
        }
    }

    #[test]
    fn test_parse_comparison() {
        let ast = QueryParser::parse("cmc:>=3").unwrap();
        match ast {
            QueryNode::Filter(filter) => {
                assert_eq!(filter.field, "cmc");
                assert_eq!(filter.operator, Operator::GreaterThanOrEqual);
                assert_eq!(filter.value, "3");
            }
            _ => panic!("Expected Filter node"),
        }
    }

    #[test]
    fn test_parse_not() {
        let ast = QueryParser::parse("not c:red").unwrap();
        match ast {
            QueryNode::Not(inner) => match *inner {
                QueryNode::Filter(filter) => {
                    assert_eq!(filter.field, "color");
                    assert_eq!(filter.value, "red");
                }
                _ => panic!("Expected Filter node inside Not"),
            },
            _ => panic!("Expected Not node"),
        }
    }
}
