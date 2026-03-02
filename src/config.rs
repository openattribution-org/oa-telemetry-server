use figment::Figment;
use figment::providers::Env;
use serde::Deserialize;

fn default_port() -> u16 {
    8007
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_session_cache_capacity() -> u64 {
    50_000
}

fn default_session_cache_ttl() -> u64 {
    3600
}

fn default_auth_cache_ttl() -> u64 {
    300
}

fn default_domain_refresh_secs() -> u64 {
    300
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub database_url: String,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_log_level")]
    pub log_level: String,

    #[serde(default = "default_session_cache_capacity")]
    pub session_cache_capacity: u64,

    #[serde(default = "default_session_cache_ttl")]
    pub session_cache_ttl: u64,

    #[serde(default = "default_auth_cache_ttl")]
    pub auth_cache_ttl: u64,

    #[serde(default = "default_domain_refresh_secs")]
    pub domain_refresh_secs: u64,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, Box<figment::Error>> {
        Figment::new()
            .merge(Env::raw())
            .extract()
            .map_err(Box::new)
    }
}
