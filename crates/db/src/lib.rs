pub mod automation;
pub mod funding;
pub mod wallet;

use std::{env, path::Path, sync::Arc};

use anyhow::Error;
use refinery::config::ConfigDbType;
use refinery_core::{Runner, load_sql_migrations};
use sqlx::PgPool;

pub fn run_db_migrations() -> Result<(), anyhow::Error> {
    dotenv::dotenv().ok();

    let db_host = env::var("DB_HOST")?;
    let db_port = env::var("DB_PORT")?;
    let db_user = env::var("DB_USER")?;
    let db_pass = env::var("DB_PASS")?;
    let db_name = env::var("DB_NAME")?;

    log::info!(
        "Running database migrations (host={db_host:?}, user={db_user:?}, port={db_port:?}, database={db_name:?})"
    );

    let mut conf = refinery::config::Config::new(ConfigDbType::Postgres)
        .set_db_user(&db_user)
        .set_db_pass(&db_pass)
        .set_db_host(&db_host)
        .set_db_port(&db_port)
        .set_db_name(&db_name);

    // Load migrations from directory at runtime
    // Use MIGRATIONS_PATH env var if set, otherwise use the default path
    let migrations_path = env::var("MIGRATIONS_PATH")
        .map(|p| Path::new(&p).to_path_buf())
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations"));
    let migrations = load_sql_migrations(&migrations_path)?;
    let runner = Runner::new(&migrations);
    runner.run(&mut conf)?;

    log::info!("✅ Migrations applied successfully.");

    Ok(())
}

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
