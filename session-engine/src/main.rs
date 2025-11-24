mod api;
mod database;
mod models;
mod sui;
mod tee;

use axum::Router;
use std::net::SocketAddr;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Initialize database
    let db = database::init().await?;
    info!("Database initialized");

    // Create application router
    let app = Router::new().merge(api::create_router()).with_state(db);

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("Server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}