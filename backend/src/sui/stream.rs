use axum::http::StatusCode;
use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine};
use sui_crypto::ed25519::Ed25519PrivateKey;
use sui_rpc::Client;
use sui_sdk_types::Address;
use sui_transaction_builder::{Function, ObjectInput};
use tracing::{info, warn};

use crate::{
    build_and_execute_tx,
    sui::constants::{ENCLAVE_CONFIG, PACKAGE, WALRUS_SYSTEM},
};

pub async fn create_stream_object(
    client: &mut Client,
    pk: &Ed25519PrivateKey,
    account_id: Address,
) -> Result<(), StatusCode> {
    info!("Creating stream object {} on Sui", account_id);

    let response = build_and_execute_tx!(client, pk, |builder| {
        let account_arg = builder.object(ObjectInput::new(account_id));
        builder.move_call(
            Function::new(
                PACKAGE.parse().unwrap(),
                "creator".parse().unwrap(),
                "start_stream".parse().unwrap(),
            ),
            vec![account_arg],
        );
    })?;

    let execution_status = response.transaction().effects().status();
    if !execution_status.success() {
        warn!("Sui transaction failed while creating stream object: {execution_status:?}");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(())
}

/// Verify the enclave signature and register the blob on Sui in a single transaction.
pub async fn verify_and_register_blob(
    client: &mut Client,
    pk: &Ed25519PrivateKey,
    account_id: Address,
    payment_amount: u64,
    stream_id: &str,
    timestamp_ms: u64,
    signature: &str,
    blob_id: &[u8],
    root_hash: &[u8],
    unencoded_size: u64,
    encoding_type: u8,
    encoded_size: u64,
    deletable: bool,
) -> Result<String, (StatusCode, String)> {
    info!(
        "Verifying signature + registering blob on Sui for stream {} (blob_id: {}, sig: {})",
        stream_id,
        URL_SAFE_NO_PAD.encode(blob_id),
        &signature[..signature.len().min(16)]
    );
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

    let stream_id_addr: Address = stream_id.parse().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid stream_id `{stream_id}` for Move `ID`: {e}"),
        )
    })?;
    let signature_bytes = URL_SAFE_NO_PAD.decode(signature).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid signature encoding, expected base64url: {e}"),
        )
    })?;

    // Move `u256` is BCS-encoded as 32 little-endian bytes.
    // Walrus/enclave blob/root hashes are handled as 32-byte big-endian digests.
    let mut blob_id_u256_le = [0u8; 32];
    blob_id_u256_le.copy_from_slice(blob_id);
    blob_id_u256_le.reverse();

    let mut root_hash_u256_le = [0u8; 32];
    root_hash_u256_le.copy_from_slice(root_hash);
    root_hash_u256_le.reverse();

    let payment_coin = todo!();

    let response = build_and_execute_tx!(client, pk, |builder| {
        let account_arg = builder.object(ObjectInput::new(account_id));
        let enclave_arg = builder.object(ObjectInput::new(ENCLAVE_CONFIG.parse().unwrap()));
        let system_arg = builder.object(ObjectInput::new(WALRUS_SYSTEM.parse().unwrap()));
        let payment_arg = builder.object(ObjectInput::new(payment_coin));
        let stream_id_arg = builder.pure(&stream_id_addr);
        let timestamp_arg = builder.pure(&timestamp_ms);
        let signature_arg = builder.pure(&signature_bytes);
        let blob_id_arg = builder.pure_bytes(blob_id_u256_le.to_vec());
        let root_hash_arg = builder.pure_bytes(root_hash_u256_le.to_vec());
        let unencoded_size_arg = builder.pure(&unencoded_size);
        let encoded_size_arg = builder.pure(&encoded_size);
        let encoding_type_arg = builder.pure(&encoding_type);
        let deletable_arg = builder.pure(&deletable);

        builder.move_call(
            Function::new(
                PACKAGE.parse().unwrap(),
                "creator".parse().unwrap(),
                "end_stream".parse().unwrap(),
            ),
            vec![
                account_arg,
                enclave_arg,
                system_arg,
                payment_arg,
                stream_id_arg,
                timestamp_arg,
                signature_arg,
                blob_id_arg,
                root_hash_arg,
                unencoded_size_arg,
                encoding_type_arg,
                encoded_size_arg,
                deletable_arg,
            ],
        );
    })
    .map_err(|status| {
        (
            status,
            "Failed to execute Sui transaction while verifying/registering blob".to_string(),
        )
    })?;

    let execution_status = response.transaction().effects().status();
    if !response.transaction().effects().status().success() {
        warn!("Sui transaction failed while verifying/registering blob: {execution_status:?}");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Sui transaction execution failed while verifying/registering blob".to_string(),
        ));
    }

    let blob_object_id = response
        .transaction()
        .effects()
        .changed_objects()
        .to_vec()
        .iter()
        .find(|obj| obj.object_type().contains("Blob"))
        .map(|obj| obj.object_id().to_string())
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "No changed object with type containing `Blob` found in transaction response"
                    .to_string(),
            )
        })?;

    Ok(blob_object_id)
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
