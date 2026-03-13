use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use sui_sdk_types::Address;
use sha2::{Digest, Sha256};

use crate::walrus::error::WalrusError;

const DEFAULT_UPLOAD_RELAY_URL: &str = "https://upload-relay.testnet.walrus.space";

pub struct TipConfig {
    pub auth_payload: Option<Vec<u8>>,
    pub tip_payment: Option<TipPayment>,
    pub nonce_b64: Option<String>,
}

pub struct TipPayment {
    pub address: Address,
    pub amount: u64,
}

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

pub async fn get_tip_config(
    payload: Vec<u8>,
    encoded_size: u64,
) -> Result<TipConfig, WalrusError> {
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
    
    let config: TipConfigResponse = response
        .json()
        .await
        .map_err(|e| WalrusError::ParseError(format!("Failed to parse tip config: {e}")))?;
    
    match config {
        TipConfigResponse::SendTip(config) => {
            let nonce: [u8; 32] = rand::rng().random();
            let blob_digest = Sha256::digest(&payload);
            let nonce_digest = Sha256::digest(&nonce);

            let mut auth = Vec::with_capacity(72);
            auth.extend_from_slice(&blob_digest);
            auth.extend_from_slice(&nonce_digest);
            auth.extend_from_slice(&(payload.len() as u64).to_le_bytes());

            let tip_amount = match &config.kind {
                TipKind::Const(v) => *v,
                TipKind::Linear {
                    base,
                    per_encoded_kib,
                } => base + per_encoded_kib * encoded_size.div_ceil(1024),
            };

            let tip_address: Address = config
                .address
                .parse()
                .map_err(|e| WalrusError::ParseError(format!("Invalid tip address: {e}")))?;

            Ok(TipConfig {
                auth_payload: Some(auth),
                tip_payment: Some(TipPayment {
                    address: tip_address,
                    amount: tip_amount,
                }),
                nonce_b64: Some(URL_SAFE_NO_PAD.encode(nonce)),
            })
        }
        TipConfigResponse::NoTip => Ok(TipConfig {
            auth_payload: None,
            tip_payment: None,
            nonce_b64: None,
        }),
    }
}