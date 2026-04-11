// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use axum::{routing::get, routing::post, Router};
use fastcrypto::{ed25519::Ed25519KeyPair, traits::KeyPair};
use session_enclave::handlers::{get_attestation, health_check};
use session_enclave::sessions::{
    close_session, get_session, open_session, revoke_session, warn_session,
};
use session_enclave::streams::{cleanup_stream, end_stream};
use session_enclave::AppState;
use redis::aio::ConnectionManager;
use redis::Client;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let eph_kp = Ed25519KeyPair::generate(&mut rand::thread_rng());
    let _internal_token = std::env::var("ENCLAVE_INTERNAL_TOKEN")
        .map_err(|_| anyhow::anyhow!("ENCLAVE_INTERNAL_TOKEN must be defined"))?;

    // let redis_password = std::env::var("REDIS_PASSWORD")
    //     .map_err(|_| anyhow::anyhow!("REDIS_PASSWORD must be defined"))?;
    // if redis_password.is_empty() {
    //     return Err(anyhow::anyhow!("REDIS_PASSWORD must not be empty"));
    // }

    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

    info!("connecting to Redis (server listens on :3000 only after this succeeds)...");
    let client = Client::open(redis_url)?;
    let redis = ConnectionManager::new(client).await?;

    // let _: String = redis::cmd("AUTH")
    //     .arg(&redis_password)
    //     .query_async(&mut redis)
    //     .await
    //     .map_err(|e| anyhow::anyhow!("Redis authentication failed: {}", e))?;

    info!("Redis connection established");

    let state = Arc::new(AppState { eph_kp, redis });

    let internal_routes = Router::new()
        .route("/sessions/open", post(open_session))
        .route("/sessions/close", post(close_session))
        .route("/sessions/warn", post(warn_session))
        .route("/sessions/revoke", post(revoke_session))
        .route("/sessions/get", post(get_session))
        .route("/streams/end", post(end_stream))
        .route("/streams/cleanup", post(cleanup_stream));

    let app = Router::new()
        .route("/", get(ping))
        .route("/attestation", get(get_attestation))
        .route("/health_check", get(health_check))
        .nest("/internal", internal_routes)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app.into_make_service())
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {e}"))
}

async fn ping() -> &'static str {
    "Pong!"
}
