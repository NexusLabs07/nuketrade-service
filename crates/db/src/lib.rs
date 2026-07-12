pub mod automation;
pub mod funding;
pub mod wallet;

use std::{env, path::Path, sync::Arc};

use anyhow::Error;
use refinery::config::{Config, ConfigDbType};
use refinery_core::{Runner, load_sql_migrations};
use sqlx::PgPool;

/// Applies all pending SQL migrations using the same DATABASE_URL consumed by
/// the runtime connection pool. Keeping one canonical connection setting avoids
/// startup failures and prevents migrations from targeting a different database.
pub fn run_db_migrations() -> Result<(), anyhow::Error> {
    dotenv::dotenv().ok();

    // Do not include the URL in an error or log message because it can contain
    // database credentials.
    let mut database_config = Config::from_env_var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL must be a valid PostgreSQL URL"))?;

    if database_config.db_type() != ConfigDbType::Postgres {
        return Err(anyhow::anyhow!(
            "DATABASE_URL must use the postgres or postgresql scheme"
        ));
    }

    log::info!("Running database migrations using DATABASE_URL");

    let migrations_path = env::var("MIGRATIONS_PATH")
        .map(|path| Path::new(&path).to_path_buf())
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations"));

    let migrations = load_sql_migrations(&migrations_path)?;
    let runner = Runner::new(&migrations);

    runner.run(&mut database_config)?;

    log::info!("Database migrations applied successfully.");

    Ok(())
}

/// Opens the shared asynchronous PostgreSQL connection pool used by the service
/// after migrations have completed.
pub async fn connect_db(db_url: &str) -> Result<Arc<PgPool>, Error> {
    let db = match sqlx::postgres::PgPool::connect(db_url).await {
        Ok(connection) => connection,
        Err(err) => {
            log::error!("Error connecting to DB: {err:?}");
            return Err(Error::msg("Error connecting to DB"));
        }
    };

    Ok(Arc::new(db))
}
