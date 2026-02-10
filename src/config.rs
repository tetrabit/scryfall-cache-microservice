use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub scryfall: ScryfallConfig,
    pub cache: CacheConfig,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_ms: u64,
    pub idle_timeout_seconds: u64,
    pub max_lifetime_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub instance_id: String,
}

#[derive(Debug, Clone)]
pub struct ScryfallConfig {
    pub rate_limit_per_second: u32,
    pub bulk_data_type: String,
    pub cache_ttl_hours: u32,
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub query_cache_ttl_hours: u32,
    pub query_cache_max_size: usize,
    pub redis: Option<RedisConfig>,
}

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
    pub ttl_seconds: u64,
    pub max_value_size_mb: usize,
    pub enabled: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Load .env file if it exists
        dotenvy::dotenv().ok();

        Ok(Config {
            database: DatabaseConfig {
                url: env::var("DATABASE_URL").context("DATABASE_URL must be set")?,
                max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .context("DATABASE_MAX_CONNECTIONS must be a valid number")?,
                min_connections: env::var("DATABASE_MIN_CONNECTIONS")
                    .unwrap_or_else(|_| "0".to_string())
                    .parse()
                    .context("DATABASE_MIN_CONNECTIONS must be a valid number")?,
                acquire_timeout_ms: env::var("DATABASE_ACQUIRE_TIMEOUT_MS")
                    .unwrap_or_else(|_| "30000".to_string())
                    .parse()
                    .context("DATABASE_ACQUIRE_TIMEOUT_MS must be a valid number")?,
                idle_timeout_seconds: env::var("DATABASE_IDLE_TIMEOUT_SECONDS")
                    .unwrap_or_else(|_| "600".to_string())
                    .parse()
                    .context("DATABASE_IDLE_TIMEOUT_SECONDS must be a valid number")?,
                max_lifetime_seconds: env::var("DATABASE_MAX_LIFETIME_SECONDS")
                    .unwrap_or_else(|_| "1800".to_string())
                    .parse()
                    .context("DATABASE_MAX_LIFETIME_SECONDS must be a valid number")?,
            },
            server: ServerConfig {
                host: env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("API_PORT")
                    .unwrap_or_else(|_| "8080".to_string())
                    .parse()
                    .context("API_PORT must be a valid port number")?,
                // Used only for debugging/observability. If unset, fall back to HOSTNAME if
                // present (e.g. Docker/Kubernetes), otherwise "unknown".
                instance_id: env::var("INSTANCE_ID")
                    .or_else(|_| env::var("HOSTNAME"))
                    .unwrap_or_else(|_| "unknown".to_string()),
            },
            scryfall: ScryfallConfig {
                rate_limit_per_second: env::var("SCRYFALL_RATE_LIMIT_PER_SECOND")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .context("SCRYFALL_RATE_LIMIT_PER_SECOND must be a valid number")?,
                bulk_data_type: env::var("SCRYFALL_BULK_DATA_TYPE")
                    .unwrap_or_else(|_| "default_cards".to_string()),
                cache_ttl_hours: env::var("SCRYFALL_CACHE_TTL_HOURS")
                    .unwrap_or_else(|_| "24".to_string())
                    .parse()
                    .context("SCRYFALL_CACHE_TTL_HOURS must be a valid number")?,
            },
            cache: CacheConfig {
                query_cache_ttl_hours: env::var("QUERY_CACHE_TTL_HOURS")
                    .unwrap_or_else(|_| "24".to_string())
                    .parse()
                    .context("QUERY_CACHE_TTL_HOURS must be a valid number")?,
                query_cache_max_size: env::var("QUERY_CACHE_MAX_SIZE")
                    .unwrap_or_else(|_| "10000".to_string())
                    .parse()
                    .context("QUERY_CACHE_MAX_SIZE must be a valid number")?,
                redis: Self::redis_config_from_env(),
            },
        })
    }

    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    fn redis_config_from_env() -> Option<RedisConfig> {
        // Redis is optional - only enabled if REDIS_ENABLED=true
        let enabled = env::var("REDIS_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        if !enabled {
            return None;
        }

        Some(RedisConfig {
            url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            ttl_seconds: env::var("REDIS_TTL_SECONDS")
                .unwrap_or_else(|_| "3600".to_string())
                .parse()
                .unwrap_or(3600),
            max_value_size_mb: env::var("REDIS_MAX_VALUE_SIZE_MB")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
            enabled: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_address() {
        let config = Config {
            database: DatabaseConfig {
                url: "postgresql://localhost/test".to_string(),
                max_connections: 10,
                min_connections: 0,
                acquire_timeout_ms: 30_000,
                idle_timeout_seconds: 600,
                max_lifetime_seconds: 1800,
            },
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
                instance_id: "test-instance".to_string(),
            },
            scryfall: ScryfallConfig {
                rate_limit_per_second: 10,
                bulk_data_type: "default_cards".to_string(),
                cache_ttl_hours: 24,
            },
            cache: CacheConfig {
                query_cache_ttl_hours: 24,
                query_cache_max_size: 10000,
                redis: None,
            },
        };

        assert_eq!(config.server_address(), "127.0.0.1:3000");
    }
}
