use std::{str::FromStr, sync::Arc};

use axum::http::StatusCode;
use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine};
use sui_crypto::ed25519::Ed25519PrivateKey;
use sui_rpc::Client;
use sui_sdk_types::{Address, StructTag, Transaction};
use sui_transaction_builder::{intent::CoinWithBalance, Function, ObjectInput, TransactionBuilder};
use tracing::{info, warn};

use crate::{
    build_and_execute_tx,
    sui::constants::{ENCLAVE_CONFIG, PACKAGE, WALRUS_SYSTEM, WAL_COIN_TYPE},
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

pub async fn build_end_stream_tx(
    client: Arc<Client>,
    sender: Address,
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
) -> Result<Transaction, (StatusCode, String)> {
    let mut client = client.as_ref().clone();
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

    let mut builder = TransactionBuilder::new();
    builder.set_sender(sender);

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

    let mut blob_id_u256_le = [0u8; 32];
    blob_id_u256_le.copy_from_slice(blob_id);
    blob_id_u256_le.reverse();

    let mut root_hash_u256_le = [0u8; 32];
    root_hash_u256_le.copy_from_slice(root_hash);
    root_hash_u256_le.reverse();

    let wal_struct_tag = StructTag::from_str(WAL_COIN_TYPE).unwrap();
    let coin_with_balance = CoinWithBalance::new(wal_struct_tag, payment_amount);

    let account_arg = builder.object(ObjectInput::new(account_id));
    let enclave_arg = builder.object(ObjectInput::new(ENCLAVE_CONFIG.parse().unwrap()));
    let system_arg = builder.object(ObjectInput::new(WALRUS_SYSTEM.parse().unwrap()));
    let payment_arg = builder.intent(coin_with_balance);
    let stream_id_arg = builder.pure(&stream_id_addr);
    let timestamp_arg = builder.pure(&timestamp_ms);
    let signature_arg = builder.pure(&signature_bytes);
    let blob_id_arg = builder.pure_bytes(blob_id_u256_le.to_vec());
    let root_hash_arg = builder.pure_bytes(root_hash_u256_le.to_vec());
    let unencoded_size_arg = builder.pure(&unencoded_size);
    let encoding_type_arg = builder.pure(&encoding_type);
    let encoded_size_arg = builder.pure(&encoded_size);
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

    builder.build(&mut client).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to build stream end tx: {err}"),
        )
    })
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
