use axum::http::StatusCode;
use tracing::{info, warn};

/// Verify the enclave signature and register the blob on Sui in a single transaction.
///
/// TODO: Replace this placeholder with a real Sui Move call that:
/// - verifies the nautilus signature
/// - registers the Walrus blob (or returns the blob/object IDs)
/// - attaches the object to the streamer account
pub async fn verify_and_register_blob_on_sui(
    stream_id: &str,
    data_hash: &str,
    signature: &str,
) -> Result<(String, String), (StatusCode, String)> {
    info!(
        "[PLACEHOLDER] Verifying signature + registering blob on Sui for stream {} (hash: {}, sig: {})",
        stream_id,
        data_hash,
        &signature[..signature.len().min(16)]
    );

    // TODO: Real Sui transaction submission.
    // - Call Move function to verify signature and register blob
    // - info! the tx digest
    // - Return the resulting blob ID and object ID
    warn!("[PLACEHOLDER] Sui verify/register not implemented");

    Ok(("object_id".to_string(), "blob_id".to_string()))
}

/// Certify a blob on Sui after receiving a confirmation certificate from the upload relay.
/// and add it to the streamer's account.
pub async fn certify_and_store_blob_on_sui(
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
pub async fn destroy_blob_on_sui(object_id: &str) -> Result<(), (StatusCode, String)> {
    warn!(
        "[PLACEHOLDER] Deleting registered blob object {} on Sui",
        object_id
    );
    // TODO: Real Sui delete call for deletable blobs.
    Ok(())
}

pub async fn flag_stream_as_invalid_on_sui(stream_id: &str) -> Result<(), (StatusCode, String)> {
    warn!(
        "[PLACEHOLDER] Flagging stream {} as invalid on Sui",
        stream_id
    );
    // TODO: Real Sui call to mark the stream as invalid.
    Ok(())
}
