use serde::{Deserialize, Serialize};

use crate::walrus::error::WalrusError;

const DEFAULT_UPLOAD_RELAY_URL: &str = "https://upload-relay.testnet.walrus.space";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipConfigResponse {
    SendTip(SendTipConfig),
    NoTip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTipConfig {
    pub address: String,
    pub kind: TipKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipKind {
    Const(u64),
    Linear { base: u64, per_encoded_kib: u64 },
}

pub async fn fetch_tip_config() -> Result<TipConfigResponse, WalrusError> {
    let url = format!("{}/v1/tip-config", DEFAULT_UPLOAD_RELAY_URL);
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(WalrusError::ApiError(status, body));
    }
    let config: TipConfigResponse = response.json().await.map_err(|e| {
        WalrusError::ParseError(format!("Failed to parse tip config: {e}"))
    })?;
    Ok(config)
}