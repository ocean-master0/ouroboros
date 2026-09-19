/// Database module — data access layer.
///
/// `pool.rs` is the only file that constructs a `PgPool`. No other module
/// calls `PgPoolOptions` directly (architecture_doc.md §5.6).
pub mod pool;

/// Run embedded SQL migrations against the database.
pub async fn run_migrations(pool: &sqlx::PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
