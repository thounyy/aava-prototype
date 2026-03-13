use std::{str::FromStr, sync::Arc};

use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine};
use sui_rpc::Client;
use sui_sdk_types::{Address, StructTag, Transaction};
use sui_transaction_builder::{Function, ObjectInput, TransactionBuilder, intent::CoinWithBalance};

use crate::sui::constants::{
    AAVA_PACKAGE, ACCOUNT_REGISTRY, ENCLAVE_CONFIG, WALRUS_SYSTEM, WAL_COIN_TYPE,
};
use crate::sui::error::SuiError;
use crate::walrus::blob::CertificateData;

pub struct TipPayment {
    pub address: Address,
    pub amount: u64,
}

// TODO: modify for production
pub async fn build_create_account_tx(
    client: Arc<Client>,
    sender: Address,
    username: String,
) -> Result<Transaction, SuiError> {
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

    builder
        .build(&mut client)
        .await
        .map_err(|e| SuiError::BuildFailed(e.to_string()))
}

pub async fn build_create_stream_tx(
    client: Arc<Client>,
    sender: Address,
    account_id: Address,
) -> Result<Transaction, SuiError> {
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

    builder
        .build(&mut client)
        .await
        .map_err(|e| SuiError::BuildFailed(e.to_string()))
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
    auth_payload: Option<Vec<u8>>,
    tip_payment: Option<TipPayment>,
) -> Result<Transaction, SuiError> {
    if blob_id.len() != 32 {
        return Err(SuiError::InvalidInput(format!(
            "blob_id must be 32 bytes, got {}",
            blob_id.len()
        )));
    }
    if root_hash.len() != 32 {
        return Err(SuiError::InvalidInput(format!(
            "root_hash must be 32 bytes, got {}",
            root_hash.len()
        )));
    }

    let mut client = client.as_ref().clone();
    let mut builder = TransactionBuilder::new();
    builder.set_sender(sender);

    // Auth payload must be input 0 for the upload relay to verify tip payment.
    if let Some(payload) = auth_payload {
        let _auth_input = builder.pure_bytes(payload);
    }

    let stream_id_addr: Address = stream_id
        .parse()
        .map_err(|e| SuiError::InvalidInput(format!("Invalid stream_id `{stream_id}`: {e}")))?;
    let signature_bytes = URL_SAFE_NO_PAD
        .decode(signature)
        .map_err(|e| SuiError::InvalidInput(format!("Invalid signature base64url: {e}")))?;

    let wal_struct_tag = StructTag::from_str(WAL_COIN_TYPE).unwrap();
    let coin_with_balance = CoinWithBalance::new(wal_struct_tag, payment_amount);

    let account_arg = builder.object(ObjectInput::new(account_id));
    let _enclave_arg = builder.object(ObjectInput::new(ENCLAVE_CONFIG.parse().unwrap()));
    let system_arg = builder.object(ObjectInput::new(WALRUS_SYSTEM.parse().unwrap()));
    let payment_arg = builder.intent(coin_with_balance);
    let stream_id_arg = builder.pure(&stream_id_addr);
    let timestamp_arg = builder.pure(&timestamp_ms);
    let signature_arg = builder.pure(&signature_bytes);
    let blob_id_arg = builder.pure_bytes(blob_id.to_vec());
    let root_hash_arg = builder.pure_bytes(root_hash.to_vec());
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
            // enclave_arg, // TODO: uncomment in production
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

    let sender_arg = builder.pure(&sender);
    builder.transfer_objects(vec![payment_arg], sender_arg);

    if let Some(tip) = tip_payment {
        let tip_amount_arg = builder.pure(&tip.amount);
        let gas = builder.gas();
        let tip_coins = builder.split_coins(gas, vec![tip_amount_arg]);
        let tip_recipient_arg = builder.pure(&tip.address);
        builder.transfer_objects(tip_coins, tip_recipient_arg);
    }

    builder
        .build(&mut client)
        .await
        .map_err(|e| SuiError::BuildFailed(e.to_string()))
}

/// Certify a blob on Sui after receiving a confirmation certificate from the upload relay.
pub async fn build_certify_blob_tx(
    client: Arc<Client>,
    sender: Address,
    account_id: Address,
    stream_id: &str,
    certificate: &CertificateData,
) -> Result<Transaction, SuiError> {
    let mut client = client.as_ref().clone();

    let stream_id_addr: Address = stream_id
        .parse()
        .map_err(|e| SuiError::InvalidInput(format!("Invalid stream_id `{stream_id}`: {e}")))?;
    let signature = &certificate.signature;
    let signers_bitmap = signers_to_bitmap(&certificate.signers)?;
    let message = &certificate.serialized_message;

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

    builder
        .build(&mut client)
        .await
        .map_err(|e| SuiError::BuildFailed(e.to_string()))
}

fn signers_to_bitmap(signers: &[u16]) -> Result<Vec<u8>, SuiError> {
    let Some(max_signer) = signers.iter().max().copied() else {
        return Err(SuiError::ParseError(
            "Confirmation certificate has no signers".into(),
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
) -> Result<Transaction, SuiError> {
    let stream_id_addr: Address = stream_id
        .parse()
        .map_err(|e| SuiError::InvalidInput(format!("Invalid stream_id `{stream_id}`: {e}")))?;
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

    builder
        .build(&mut client)
        .await
        .map_err(|e| SuiError::BuildFailed(format!("destroy_blob for {stream_id}: {e}")))
}
