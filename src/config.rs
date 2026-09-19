/// Application configuration loaded from environment variables via figment.
///
/// All fields have sensible defaults except `database_url`, which must be
/// provided via `DATABASE_URL` in the environment or `.env` file.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    /// PostgreSQL connection string (required).
    pub database_url: String,
    /// TCP port to bind (default 8080).
    #[serde(default = "default_port")]
    pub port: u16,
    /// Maximum PostgreSQL connection pool size (default 100).
    #[serde(default = "default_max_db_connections")]
    pub max_db_connections: u32,
    /// Maximum concurrency for the built-in bench engine (default 2000).
    #[serde(default = "default_bench_max_concurrency")]
    pub bench_max_concurrency: u32,
    /// Maximum request body size in bytes (default 50MB).
    #[serde(default = "default_body_limit_bytes")]
    pub body_limit_bytes: usize,
    /// Per-request timeout in seconds (default 30).
    #[serde(default = "default_request_timeout_secs")]
    pub request_timeout_secs: u64,
    /// Log level filter string (default "info").
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Config {
    /// Load configuration from environment variables (with optional `.env` file).
    ///
    /// Fails fast with a clear error if required fields are missing.
    pub fn load() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok(); // .env is optional in prod, required in dev
        figment::Figment::new()
            .merge(figment::providers::Env::raw())
            .extract()
            .map_err(|e| anyhow::anyhow!("config load failed: {e}"))
    }
}

fn default_port() -> u16 {
    8080
}
fn default_max_db_connections() -> u32 {
    100
}
fn default_bench_max_concurrency() -> u32 {
    2000
}
fn default_body_limit_bytes() -> usize {
    50 * 1024 * 1024 // 50 MB
}
fn default_request_timeout_secs() -> u64 {
    30
}
fn default_log_level() -> String {
    "info".to_string()
}
