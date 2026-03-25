use axum::http::header;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::routing::get;
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
        .route("/docs", get(swagger_ui))
        .route("/openapi.json", get(openapi_json))
        .merge(api::creator::create_router())
        .merge(api::viewer::create_router())
        .merge(api::sessions::create_router())
        .merge(api::streams::create_router())
        .merge(api::actions::create_router())
        .with_state(state);

    let host = std::env::var("SESSION_ENGINE_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port: u16 = std::env::var("SESSION_ENGINE_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid SESSION_ENGINE_HOST/SESSION_ENGINE_PORT: {e}"))?;
    info!("Server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn swagger_ui() -> impl IntoResponse {
    // Swagger UI is loaded from a CDN. If your colleague is behind a network policy
    // that blocks external access, we can bundle the static files instead.
    Html::from(OPENAPI_UI_HTML)
}

async fn openapi_json() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json")],
        include_str!("../openapi.json"),
    )
}

/// Minimal Swagger UI page pointing at `/openapi.json`.
///
/// Note: uses relative path so it works regardless of host/port.
const OPENAPI_UI_HTML: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Aava API Docs</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist/swagger-ui.css" />
  </head>
  <body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist/swagger-ui-bundle.js"></script>
    <script src="https://unpkg.com/swagger-ui-dist/swagger-ui-standalone-preset.js"></script>
    <script>
      window.onload = () => {
        SwaggerUIBundle({
          url: '/openapi.json',
          dom_id: '#swagger-ui',
          presets: [
            SwaggerUIBundle.presets.apis,
            SwaggerUIStandalonePreset
          ],
          layout: 'BaseLayout'
        });
      };
    </script>
  </body>
</html>
"#;
