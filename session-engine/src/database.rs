use sqlx::{PgPool, postgres::PgPoolOptions};
use anyhow::Result;
use tracing::info;

const DATABASE_URL: &str = "postgresql:///aava";

pub async fn init() -> Result<PgPool> {
    // Create connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(DATABASE_URL)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;
    
    info!("Database migrations completed");
    
    Ok(pool)
}

pub type DbPool = PgPool;





