use axum::http::StatusCode;
use tracing::{info, warn};

/// Verify the enclave signature and register the blob on Sui in a single transaction.
///
/// TODO: Replace this placeholder with a real Sui Move call that:
/// - verifies the nautilus signature
/// - registers the Walrus blob (or returns the blob/object IDs)
/// - attaches the object to the streamer account
pub async fn verify_and_register_dataset(
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
pub async fn certify_blob_on_sui(
    blob_id: &str,
    confirmation_certificate: &serde_json::Value,
) -> Result<(), (StatusCode, String)> {
    warn!(
        "[PLACEHOLDER] Certifying blob {} on Sui with confirmation certificate",
        blob_id
    );
    // TODO: Real Sui call to certify the blob using the certificate.
    let _ = confirmation_certificate;
    Ok(())
}

/// Cleanup helper for cases where Sui registration succeeded but Walrus upload failed.
pub async fn delete_registered_blob(blob_object_id: &str) -> Result<(), (StatusCode, String)> {
    warn!(
        "[PLACEHOLDER] Deleting registered blob object {} on Sui",
        blob_object_id
    );
    // TODO: Real Sui delete call for deletable blobs.
    Ok(())
}
