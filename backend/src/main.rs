use axum::Router;
use session_engine::{AppState, api};
use std::net::SocketAddr;
use std::sync::Arc;
use sui_rpc::Client;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let sui_rpc_url =
        std::env::var("SUI_RPC_URL").unwrap_or_else(|_| Client::TESTNET_FULLNODE.to_string());
    let sui_client = Client::new(sui_rpc_url.as_str())?;
    let state = Arc::new(AppState { sui_client });

    // Create application router
    let app = Router::new()
        .merge(api::viewers::create_router())
        .merge(api::sessions::create_router())
        .merge(api::streams::create_router())
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    info!("Server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
