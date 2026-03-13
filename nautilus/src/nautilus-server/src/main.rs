// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use axum::{routing::get, routing::post, Router};
use fastcrypto::{ed25519::Ed25519KeyPair, traits::KeyPair};
use nautilus_server::app::{cleanup_stream, close_session, end_stream, open_session};
use nautilus_server::common::{get_attestation, health_check};
use nautilus_server::AppState;
use redis::aio::ConnectionManager;
use redis::Client;
use std::sync::Arc;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {    
    let eph_kp = Ed25519KeyPair::generate(&mut rand::thread_rng());
    let _internal_token = std::env::var("ENCLAVE_INTERNAL_TOKEN")
        .map_err(|_| anyhow::anyhow!("ENCLAVE_INTERNAL_TOKEN must be defined"))?;

    // Initialize Redis connection
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

    let client = Client::open(redis_url.clone())?;
    let mut redis = ConnectionManager::new(client).await?;

    // Authenticate with Redis
    // Priority: REDIS_PASSWORD env var > password in URL
    // REDIS_PASSWORD is more secure (doesn't show in process list)
    if let Ok(password) = std::env::var("REDIS_PASSWORD") {
        if !password.is_empty() {
            let _: String = redis::cmd("AUTH")
                .arg(&password)
                .query_async(&mut redis)
                .await
                .map_err(|e| anyhow::anyhow!("Redis authentication failed: {}", e))?;
            info!("Redis authentication successful (using REDIS_PASSWORD)");
        }
    } else if redis_url.contains("@") && redis_url.contains(":") {
        // Password might be in URL format: redis://:password@host:port
        // The redis crate should handle this, but verify connection works
        let _: String = redis::cmd("PING")
            .query_async(&mut redis)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Redis connection test failed (check password in URL): {}",
                    e
                )
            })?;
        info!("Redis connection verified (password from URL)");
    } else {
        // No password - test connection (will fail if Redis requires auth)
        let _: String = redis::cmd("PING")
            .query_async(&mut redis)
            .await
            .map_err(|e| anyhow::anyhow!("Redis connection failed. If Redis requires authentication, set REDIS_PASSWORD env var or include password in REDIS_URL: {}", e))?;
        warn!(
            "Redis connection established WITHOUT authentication - NOT RECOMMENDED for production!"
        );
    }

    info!("Redis connection established and authenticated");

    let state = Arc::new(AppState { eph_kp, redis });

    let internal_routes = Router::new()
        .route("/sessions/open", post(open_session))
        .route("/sessions/close", post(close_session))
        .route("/streams/end", post(end_stream))
        .route("/streams/cleanup", post(cleanup_stream));

    let app = Router::new()
        .route("/", get(ping))
        .route("/get_attestation", get(get_attestation))
        .route("/health_check", get(health_check))
        .nest("/internal", internal_routes)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app.into_make_service())
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {e}"))
}

async fn ping() -> &'static str {
    "Pong!"
}
