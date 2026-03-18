use std::collections::HashMap;
use std::sync::Arc;

use prost_types::value::Kind;
use sui_rpc::Client;
use sui_sdk_types::{Address, Transaction};
use sui_transaction_builder::{Function, ObjectInput, TransactionBuilder};

use crate::sui::constants::{AAVA_PACKAGE, ACCOUNT_REGISTRY, PROTOCOL_AUTHORITY};
use crate::sui::error::SuiError;
use crate::sui::read;

pub async fn account_exists(client: Arc<Client>, account_id: Address) -> Result<bool, SuiError> {
    let obj = read::get_object(client, account_id).await?;
    Ok(obj
        .object_type_opt()
        .is_some_and(|t| t.contains(&format!("{}::viewer::Account", AAVA_PACKAGE))))
}

pub async fn get_account(
    client: Arc<Client>,
    account_id: Address,
) -> Result<(Option<String>, HashMap<String, String>), SuiError> {
    let obj = read::get_object(client, account_id).await?;

    if !obj
        .object_type_opt()
        .is_some_and(|t| t == &format!("{}::viewer::Account", AAVA_PACKAGE))
    {
        return Err(SuiError::InvalidInput(format!(
            "Object {account_id} is not a viewer::Account (type={})",
            obj.object_type_opt().unwrap_or_default()
        )));
    }

    let fields = match obj.json_opt().and_then(|v| v.kind.as_ref()) {
        Some(Kind::StructValue(s)) => &s.fields,
        _ => {
            return Err(SuiError::ParseError(format!(
                "Missing JSON fields for viewer::Account {account_id}"
            )))
        }
    };

    let addr = match fields.get("addr").and_then(|v| v.kind.as_ref()) {
        None | Some(Kind::NullValue(_)) => None,
        Some(Kind::StringValue(s)) => Some(s.to_owned()),
        Some(other) => {
            return Err(SuiError::ParseError(format!(
                "Unexpected addr kind for viewer::Account: {other:?}"
            )))
        }
    };

    let metadata = fields
        .get("metadata")
        .and_then(|m| m.kind.as_ref())
        .and_then(|k| match k {
            Kind::StructValue(s) => s.fields.get("contents"),
            _ => None,
        })
        .and_then(|contents| contents.kind.as_ref())
        .and_then(|k| match k {
            Kind::ListValue(list) => Some(list),
            _ => None,
        })
        .map(|list| {
            list.values
                .iter()
                .filter_map(|item| match item.kind.as_ref() {
                    Some(Kind::StructValue(entry)) => {
                        let key = entry
                            .fields
                            .get("key")
                            .and_then(|v| match v.kind.as_ref() {
                                Some(Kind::StringValue(s)) => Some(s.clone()),
                                _ => None,
                            })?;
                        let val =
                            entry
                                .fields
                                .get("value")
                                .and_then(|v| match v.kind.as_ref() {
                                    Some(Kind::StringValue(s)) => Some(s.clone()),
                                    _ => None,
                                })?;
                        Some((key, val))
                    }
                    _ => None,
                })
                .collect::<HashMap<String, String>>()
        })
        .unwrap_or_default();

    Ok((addr, metadata))
}

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
