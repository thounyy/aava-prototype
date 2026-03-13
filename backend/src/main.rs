use axum::Router;
use session_engine::{AppState, api, sui};
use std::net::SocketAddr;
use std::sync::Arc;
use sui_rpc::Client;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    info!("Server wallet address: {}", sui::executor::wallet_address());
    let _internal_token = std::env::var("ENCLAVE_INTERNAL_TOKEN")
        .map_err(|_| anyhow::anyhow!("ENCLAVE_INTERNAL_TOKEN must be defined"))?;

    let sui_client = Client::new(Client::TESTNET_FULLNODE)?;
    let state = Arc::new(AppState {
        sui_client: Arc::new(sui_client),
    });

    let app = Router::new()
        .merge(api::accounts::create_router())
        .merge(api::sessions::create_router())
        .merge(api::streams::create_router())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    info!("Server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
