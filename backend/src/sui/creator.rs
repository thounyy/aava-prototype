use std::{str::FromStr, sync::Arc};

use axum::http::StatusCode;
use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine};
use sui_rpc::Client;
use sui_sdk_types::{Address, StructTag, Transaction};
use sui_transaction_builder::{intent::CoinWithBalance, Function, ObjectInput, TransactionBuilder};
use walrus_core::messages::ConfirmationCertificate;

use crate::sui::constants::{
    AAVA_PACKAGE, ACCOUNT_REGISTRY, ENCLAVE_CONFIG, WALRUS_SYSTEM, WAL_COIN_TYPE,
};

// TODO: modify for production
pub async fn build_create_account_tx(
    client: Arc<Client>,
    sender: Address,
    username: String,
) -> Result<Transaction, (StatusCode, String)> {
    let mut client = client.as_ref().clone();
    let mut builder = TransactionBuilder::new();
    builder.set_sender(sender);

    let registry_arg = builder.object(ObjectInput::new(ACCOUNT_REGISTRY.parse().unwrap()));
    let addr_arg = builder.pure(&sender);
    let username_arg = builder.pure(&username);

    builder.move_call(
        Function::new(
            AAVA_PACKAGE.parse().unwrap(),
            "creator".parse().unwrap(),
            "new_account_for_testing".parse().unwrap(),
        ),
        vec![registry_arg, addr_arg, username_arg],
    );

    builder.build(&mut client).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to build start stream tx: {err}"),
        )
    })
}

pub async fn build_create_stream_tx(
    client: Arc<Client>,
    sender: Address,
    account_id: Address,
) -> Result<Transaction, (StatusCode, String)> {
    let mut client = client.as_ref().clone();
    let mut builder = TransactionBuilder::new();
    builder.set_sender(sender);

    let account_arg = builder.object(ObjectInput::new(account_id));
    builder.move_call(
        Function::new(
            AAVA_PACKAGE.parse().unwrap(),
            "creator".parse().unwrap(),
            "create_stream".parse().unwrap(),
        ),
        vec![account_arg],
    );

    builder.build(&mut client).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to build start stream tx: {err}"),
        )
    })
}

pub async fn build_verify_and_store_blob_tx(
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
            AAVA_PACKAGE.parse().unwrap(),
            "creator".parse().unwrap(),
            "verify_and_store_blob".parse().unwrap(),
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
pub async fn build_certify_blob_tx(
    client: Arc<Client>,
    sender: Address,
    account_id: Address,
    stream_id: &str,
    confirmation_certificate: &ConfirmationCertificate,
) -> Result<Transaction, (StatusCode, String)> {
    let mut client = client.as_ref().clone();

    let stream_id_addr: Address = stream_id.parse().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid stream_id `{stream_id}` for Move `ID`: {e}"),
        )
    })?;
    let signature = &confirmation_certificate.signature.as_ref().to_vec();
    let signers_bitmap = signers_to_bitmap(&confirmation_certificate.signers)?;
    let message = &confirmation_certificate.serialized_message;

    let mut builder = TransactionBuilder::new();
    builder.set_sender(sender);

    let account_arg = builder.object(ObjectInput::new(account_id));
    let system_arg = builder.object(ObjectInput::new(WALRUS_SYSTEM.parse().unwrap()));
    let stream_id_arg = builder.pure(&stream_id_addr);
    let signature_arg = builder.pure(signature);
    let signers_bitmap_arg = builder.pure(&signers_bitmap);
    let message_arg = builder.pure(message);

    builder.move_call(
        Function::new(
            AAVA_PACKAGE.parse().unwrap(),
            "creator".parse().unwrap(),
            "certify_blob".parse().unwrap(),
        ),
        vec![
            account_arg,
            system_arg,
            stream_id_arg,
            signature_arg,
            signers_bitmap_arg,
            message_arg,
        ],
    );

    builder.build(&mut client).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to build certify_blob tx: {err}"),
        )
    })
}

fn signers_to_bitmap(signers: &[u16]) -> Result<Vec<u8>, (StatusCode, String)> {
    let Some(max_signer) = signers.iter().max().copied() else {
        return Err((
            StatusCode::BAD_GATEWAY,
            "Confirmation certificate has no signers".to_string(),
        ));
    };
    let mut bitmap = vec![0u8; usize::from(max_signer / 8 + 1)];
    for signer in signers {
        let byte_index = usize::from(*signer / 8);
        let bit_index = (*signer % 8) as u8;
        bitmap[byte_index] |= 1u8 << bit_index;
    }
    Ok(bitmap)
}

/// Cleanup helper for cases where Sui registration succeeded but Walrus upload failed.
pub async fn build_destroy_blob_tx(
    client: Arc<Client>,
    sender: Address,
    account_id: Address,
    stream_id: &str,
) -> Result<Transaction, (StatusCode, String)> {
    let stream_id_addr: Address = stream_id.parse().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid stream_id `{stream_id}` for Move `ID`: {e}"),
        )
    })?;
    let mut client = client.as_ref().clone();
    let mut builder = TransactionBuilder::new();
    builder.set_sender(sender);

    let account_arg = builder.object(ObjectInput::new(account_id));
    let system_arg = builder.object(ObjectInput::new(WALRUS_SYSTEM.parse().unwrap()));
    let stream_id_arg = builder.pure(&stream_id_addr);
    builder.move_call(
        Function::new(
            AAVA_PACKAGE.parse().unwrap(),
            "creator".parse().unwrap(),
            "destroy_blob".parse().unwrap(),
        ),
        vec![account_arg, system_arg, stream_id_arg],
    );

    builder.build(&mut client).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to build destroy_blob tx for stream {stream_id}: {err}"),
        )
    })
}
