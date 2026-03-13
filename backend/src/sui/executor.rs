use std::{sync::Arc, time::Duration};

use sui_crypto::{SuiSigner, ed25519::Ed25519PrivateKey};
use sui_rpc::{
    Client,
    field::{FieldMask, FieldMaskUtil},
    proto::sui::rpc::v2::ExecuteTransactionRequest,
};
use sui_sdk_types::{Address, Transaction};
use tracing::info;

use crate::sui::error::SuiError;

// TODO: replace with proper key management
const WALLET_SECRET: [u8; 32] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32,
];

pub fn wallet_key() -> Ed25519PrivateKey {
    Ed25519PrivateKey::new(WALLET_SECRET)
}

pub fn wallet_address() -> Address {
    wallet_key().public_key().derive_address()
}

pub struct ExecutedTx {
    pub digest: String,
}

/// Sign a transaction with the server wallet key, execute it, and wait for checkpoint inclusion.
pub async fn sign_and_execute(
    client: Arc<Client>,
    tx: Transaction,
) -> Result<ExecutedTx, SuiError> {
    let digest = tx.digest().to_string();
    let key = wallet_key();

    let user_sig = key
        .sign_transaction(&tx)
        .map_err(|e| SuiError::SignFailed(e.to_string()))?;

    let proto_tx: sui_rpc::proto::sui::rpc::v2::Transaction = tx.into();
    let proto_sig: sui_rpc::proto::sui::rpc::v2::UserSignature = user_sig.into();

    let mut request = ExecuteTransactionRequest::new(proto_tx);
    request.signatures = vec![proto_sig];
    request.read_mask = Some(FieldMask::from_str("effects.status,effects.changed_objects"));

    let mut rpc = client.as_ref().clone();
    let response = rpc
        .execute_transaction_and_wait_for_checkpoint(request, Duration::from_secs(60))
        .await
        .map_err(|e| SuiError::RpcError(format!("Execution failed for {digest}: {e}")))?;

    let executed = response.into_inner();
    let status = executed.transaction().effects().status();
    if !status.success() {
        return Err(SuiError::OnChainFailed(format!(
            "Transaction {digest} failed: {status:?}"
        )));
    }

    info!("Transaction {digest} executed and checkpointed");
    Ok(ExecutedTx { digest })
}
