use axum::http::StatusCode;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use tracing::{info, warn};

/// Verify the enclave signature and register the blob on Sui in a single transaction.
///
/// TODO: Replace this placeholder with a real Sui Move call that:
/// - verifies the nautilus signature
/// - registers the Walrus blob (or returns the blob/object IDs)
/// - attaches the object to the streamer account
pub async fn verify_and_register_blob(
    stream_id: &str,
    blob_id: &[u8],
    root_hash: &[u8],
    size: u64,
    encoding_type: u8,
    deletable: bool,
    timestamp_ms: u64,
    signature: &str,
) -> Result<String, (StatusCode, String)> {
    if blob_id.len() != 32 {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Invalid blob_id length {}, expected 32 bytes",
                blob_id.len()
            ),
        ));
    }
    if root_hash.len() != 32 {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Invalid root_hash length {}, expected 32 bytes",
                root_hash.len()
            ),
        ));
    }

    info!(
        "[PLACEHOLDER] Verifying signature + registering blob on Sui for stream {} (blob_id: {}, sig: {})",
        stream_id,
        URL_SAFE_NO_PAD.encode(blob_id),
        &signature[..signature.len().min(16)]
    );

    let object_id = call_creator_end_stream(
        stream_id,
        blob_id,
        root_hash,
        size,
        encoding_type,
        deletable,
        timestamp_ms,
        signature,
    )
    .await?;

    Ok(object_id)
}

async fn call_creator_end_stream(
    stream_id: &str,
    blob_id: &[u8],
    root_hash: &[u8],
    size: u64,
    encoding_type: u8,
    deletable: bool,
    timestamp_ms: u64,
    signature: &str,
) -> Result<String, (StatusCode, String)> {
    warn!(
        "[PLACEHOLDER] Calling aava::creator::end_stream on Sui for stream {}",
        stream_id
    );
    // TODO: Real Sui transaction submission.
    // Required Move args (from creator.move):
    // - account, enclave, system, storage
    // - stream_id
    // - blob_id
    // - timestamp_ms
    // - signature
    // - blob_id (u256), root_hash (u256), size (u64), encoding_type (u8), deletable (bool)
    // - write_payment (Coin<WAL>)
    // - ctx
    let _ = (
        blob_id,
        root_hash,
        size,
        encoding_type,
        deletable,
        timestamp_ms,
        signature,
    );

    Ok("object_id".to_string())
}

/// Certify a blob on Sui after receiving a confirmation certificate from the upload relay.
/// and add it to the streamer's account.
pub async fn certify_and_store_blob(
    object_id: &str,
    blob_id: &str,
    confirmation_certificate: &serde_json::Value,
) -> Result<(), (StatusCode, String)> {
    warn!(
        "[PLACEHOLDER] Certifying blob {} (obj_id: {}) on Sui with confirmation certificate",
        object_id, blob_id
    );
    // TODO: Real Sui call to certify the blob using the certificate.
    let _ = confirmation_certificate;
    Ok(())
}

/// Cleanup helper for cases where Sui registration succeeded but Walrus upload failed.
pub async fn destroy_blob(object_id: &str) -> Result<(), (StatusCode, String)> {
    warn!(
        "[PLACEHOLDER] Deleting registered blob object {} on Sui",
        object_id
    );
    // TODO: Real Sui delete call for deletable blobs.
    Ok(())
}

pub async fn flag_stream_as_invalid(stream_id: &str) -> Result<(), (StatusCode, String)> {
    warn!(
        "[PLACEHOLDER] Flagging stream {} as invalid on Sui",
        stream_id
    );
    // TODO: Real Sui call to mark the stream as invalid.
    Ok(())
}
