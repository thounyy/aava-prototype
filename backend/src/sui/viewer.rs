use std::sync::Arc;

use sui_rpc::Client;
use sui_sdk_types::{Address, Transaction};
use sui_transaction_builder::{Function, ObjectInput, TransactionBuilder};

use crate::sui::constants::{AAVA_PACKAGE, ACCOUNT_REGISTRY, PROTOCOL_AUTHORITY};
use crate::sui::error::SuiError;

// TODO: modify with batching and queuing viewer account creations
pub async fn build_create_account_tx(
    client: Arc<Client>,
    sender: Address,
    username: String,
) -> Result<Transaction, SuiError> {
    let mut client = client.as_ref().clone();
    let mut builder = TransactionBuilder::new();
    builder.set_sender(sender);

    // call 1: get auth
    let protocol_authority_arg =
        builder.object(ObjectInput::new(PROTOCOL_AUTHORITY.parse().unwrap()));

    let auth = builder.move_call(
        Function::new(
            AAVA_PACKAGE.parse().unwrap(),
            "protocol_authority".parse().unwrap(),
            "init_viewers".parse().unwrap(),
        ),
        vec![protocol_authority_arg],
    );

    // call 2: create account
    let registry_arg = builder.object(ObjectInput::new(ACCOUNT_REGISTRY.parse().unwrap()));
    let username_arg = builder.pure(&username);

    builder.move_call(
        Function::new(
            AAVA_PACKAGE.parse().unwrap(),
            "viewer".parse().unwrap(),
            "new_account".parse().unwrap(),
        ),
        vec![registry_arg, auth, username_arg],
    );

    // call 3: destroy auth
    builder.move_call(
        Function::new(
            AAVA_PACKAGE.parse().unwrap(),
            "protocol_authority".parse().unwrap(),
            "finalize_viewers".parse().unwrap(),
        ),
        vec![auth],
    );

    builder
        .build(&mut client)
        .await
        .map_err(|e| SuiError::BuildFailed(e.to_string()))
}
