pub mod crud;
pub mod types;

use std::{env, sync::Arc};

use anyhow::Error;
use refinery::config::ConfigDbType;
use sqlx::PgPool;

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./migrations");
}

pub fn run_db_migrations() -> Result<(), anyhow::Error> {
    dotenv::dotenv().ok();

    let db_host = env::var("DB_HOST")?;
    let db_port = env::var("DB_PORT")?;
    let db_user = env::var("DB_USER")?;
    let db_pass = env::var("DB_PASS")?;
    let db_name = env::var("DB_NAME")?;

    log::info!(
        "db_host: {:?}, db_post {:?}, db_user {:?}, db_pass {:?}, db_name {:?}",
        db_host,
        db_pass,
        db_user,
        db_pass,
        db_name
    );

    let mut conf = refinery::config::Config::new(ConfigDbType::Postgres)
        .set_db_user(&db_user)
        .set_db_pass(&db_pass)
        .set_db_host(&db_host)
        .set_db_port(&db_port)
        .set_db_name(&db_name);

    // Apply embedded migrations using sqlx pool
    let _ = embedded::migrations::runner().run(&mut conf)?;

    log::info!("✅ Migrations applied successfully.");

    Ok(())
}

pub async fn connect_db(db_url: &str) -> Result<Arc<PgPool>, Error> {
    let db = match sqlx::postgres::PgPool::connect(db_url).await {
        Ok(connection) => connection,
        Err(err) => {
            log::error!("Error connecting to DB: {:?}", err);
            return Err(Error::msg("Error connecting to DB"));
        }
    };

    Ok(Arc::new(db))
}
