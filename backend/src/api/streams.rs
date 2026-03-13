use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::Json,
    routing::post,
    Router,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sui_sdk_types::Address;
use tracing::{info, warn};

use crate::enclave;
use crate::error::AppError;
use crate::sui;
use crate::walrus;
use crate::AppState;

// From walrus-sui — 1 MiB per storage unit.
pub const BYTES_PER_UNIT_SIZE: u64 = 1_024 * 1_024;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/creators/{account_handle}/streams",
            post(start_stream),
        )
        .route(
            "/api/creators/{account_handle}/streams/{stream_id}/end",
            post(end_stream),
        )
}

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStartResponse {
    pub tx_digest: String,
    pub stream_account_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEndResponse {
    pub stream_id: String,
    pub sessions_count: u64,
    pub init_tx_digest: String,
    pub finalize_action: String,
    pub finalize_tx_digest: String,
}

// ── Handlers ────────────────────────────────────────────────────────

async fn start_stream(
    State(state): State<Arc<AppState>>,
    Path(account_handle): Path<String>,
) -> Result<Json<StreamStartResponse>, AppError> {
    let account_id = sui::read::derive_account_id(&account_handle)?;
    info!(
        "Creating stream for account_handle {} (derived account {})",
        account_handle, account_id
    );

    let tx = sui::creator::build_create_stream_tx(
        state.sui_client.clone(),
        sui::executor::wallet_address(),
        account_id,
    )
    .await?;

    let result = sui::executor::sign_and_execute(state.sui_client.clone(), tx).await?;

    Ok(Json(StreamStartResponse {
        tx_digest: result.digest,
        stream_account_id: account_id.to_string(),
    }))
}

/// 1. Fetch attested session data from enclave
/// 2. Build, sign & execute the verify_and_store_blob tx
/// 3. Upload blob payload to Walrus relay
/// 4. Build, sign & execute the certify (or destroy) blob tx
/// 5. Cleanup enclave state
async fn end_stream(
    State(state): State<Arc<AppState>>,
    Path((account_handle, stream_id)): Path<(String, String)>,
) -> Result<Json<StreamEndResponse>, AppError> {
    let account_id = sui::read::derive_account_id(&account_handle)?;
    info!(
        "Ending stream {} for account_handle {} (derived account {})",
        stream_id, account_handle, account_id
    );

    // ── 1. Fetch Walrus system params ───────────────────────────────
    let walrus_params = sui::read::fetch_walrus_system_params(state.sui_client.clone()).await?;

    // ── 2. Fetch attested sessions from enclave ─────────────────────
    let (data, signature, timestamp_ms) =
        enclave::stream::fetch_signed_dataset(&stream_id, walrus_params.n_shards).await?;

    if data.sessions_count == 0 {
        warn!("No active sessions found for stream {}", stream_id);
        return Ok(Json(StreamEndResponse {
            stream_id,
            sessions_count: 0,
            init_tx_digest: String::new(),
            finalize_action: "noop".into(),
            finalize_tx_digest: String::new(),
        }));
    }

    // ── 3. Prepare payload & tip ────────────────────────────────────
    let payload = serde_json::to_vec(&data.sessions).map_err(|e| {
        AppError::Internal(format!("Failed to serialize Walrus payload: {e}"))
    })?;

    let price_for_encoded_length =
        data.encoded_size.div_ceil(BYTES_PER_UNIT_SIZE) * walrus_params.price_per_unit_size * 53;

    let tip_config = walrus::tip::fetch_tip_config().await?;

    let (auth_payload, tip_payment, nonce_b64) = match &tip_config {
        walrus::tip::TipConfigResponse::SendTip(config) => {
            let nonce: [u8; 32] = rand::rng().random();
            let blob_digest = Sha256::digest(&payload);
            let nonce_digest = Sha256::digest(&nonce);

            let mut auth = Vec::with_capacity(72);
            auth.extend_from_slice(&blob_digest);
            auth.extend_from_slice(&nonce_digest);
            auth.extend_from_slice(&(payload.len() as u64).to_le_bytes());

            let tip_amount = match &config.kind {
                walrus::tip::TipKind::Const(v) => *v,
                walrus::tip::TipKind::Linear { base, per_encoded_kib } => {
                    base + per_encoded_kib * data.encoded_size.div_ceil(1024)
                }
            };

            let tip_address: Address = config
                .address
                .parse()
                .map_err(|e| AppError::BadRequest(format!("Invalid tip address: {e}")))?;

            (
                Some(auth),
                Some(sui::creator::TipPayment {
                    address: tip_address,
                    amount: tip_amount,
                }),
                Some(URL_SAFE_NO_PAD.encode(nonce)),
            )
        }
        walrus::tip::TipConfigResponse::NoTip => (None, None, None),
    };

    // ── 4. Build & execute verify_and_store_blob tx ─────────────────
    let sender = sui::executor::wallet_address();

    let register_tx = sui::creator::build_verify_and_store_blob_tx(
        state.sui_client.clone(),
        sender,
        account_id,
        price_for_encoded_length,
        &stream_id,
        timestamp_ms,
        &signature,
        &data.blob_id,
        &data.root_hash,
        data.unencoded_size,
        data.encoding_type.into(),
        data.encoded_size,
        true,
        auth_payload,
        tip_payment,
    )
    .await?;

    let register_result =
        sui::executor::sign_and_execute(state.sui_client.clone(), register_tx).await?;

    info!(
        "Register blob tx executed for stream {}: {}",
        stream_id, register_result.digest
    );

    // ── 5. Find the Blob object from tx effects ─────────────────────
    let blob_object_id =
        find_blob_object_id(state.sui_client.clone(), &register_result.digest).await?;

    let blob_id_b64 = URL_SAFE_NO_PAD.encode(&data.blob_id);

    // ── 6. Upload to Walrus relay, then certify or destroy ──────────
    let (finalize_action, finalize_tx) = match walrus::blob::upload_dataset(
        &blob_object_id,
        &blob_id_b64,
        payload,
        Some(&register_result.digest),
        nonce_b64.as_deref(),
    )
    .await
    {
        Ok(relay_response) => {
            let tx = sui::creator::build_certify_blob_tx(
                state.sui_client.clone(),
                sender,
                account_id,
                &stream_id,
                &relay_response.certificate,
            )
            .await?;
            ("certify_blob".to_string(), tx)
        }
        Err(e) => {
            warn!(
                "Walrus upload failed after successful tx {} for stream {}: {e}",
                register_result.digest, stream_id,
            );
            let tx = sui::creator::build_destroy_blob_tx(
                state.sui_client.clone(),
                sender,
                account_id,
                &stream_id,
            )
            .await?;
            ("destroy_blob".to_string(), tx)
        }
    };

    let finalize_result =
        sui::executor::sign_and_execute(state.sui_client.clone(), finalize_tx).await?;

    info!(
        "Finalize ({finalize_action}) tx executed for stream {}: {}",
        stream_id, finalize_result.digest
    );

    enclave::stream::cleanup_dataset(&stream_id).await?;

    Ok(Json(StreamEndResponse {
        stream_id,
        sessions_count: data.sessions_count,
        init_tx_digest: register_result.digest,
        finalize_action,
        finalize_tx_digest: finalize_result.digest,
    }))
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Extract the Blob object ID from the effects of an executed tx.
async fn find_blob_object_id(
    client: Arc<sui_rpc::Client>,
    tx_digest: &str,
) -> Result<String, AppError> {
    use sui_rpc::field::{FieldMask, FieldMaskUtil};
    use sui_rpc::proto::sui::rpc::v2::GetTransactionRequest;

    let request = GetTransactionRequest::default()
        .with_digest(tx_digest)
        .with_read_mask(FieldMask::from_str("effects.changed_objects"));

    let response = client
        .as_ref()
        .clone()
        .ledger_client()
        .get_transaction(request)
        .await
        .map_err(|e| {
            sui::error::SuiError::RpcError(format!("Failed to fetch tx {tx_digest}: {e}"))
        })?;

    let tx = response.into_inner().transaction().clone();
    tx.effects()
        .changed_objects()
        .iter()
        .find(|obj| obj.object_type().contains("Blob"))
        .map(|obj| obj.object_id().to_string())
        .ok_or_else(|| {
            AppError::Internal(format!("No Blob object found in tx {tx_digest}"))
        })
}
