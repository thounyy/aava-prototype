use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sui_rpc::Client;
use sui_sdk_types::{Address, Transaction};
use sui_transaction_builder::{Function, ObjectInput, TransactionBuilder};

use crate::sui::constants::{AAVA_PACKAGE, ACCOUNT_REGISTRY, PROTOCOL_AUTHORITY};
use crate::sui::error::SuiError;
use crate::sui::read;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewerAccount {
    pub id: String,
    pub handle: String,
    pub owner: Option<String>,
    pub protocol: String,
    pub sanctions: Vec<ViewerSanction>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewerSanction {
    pub session_id: String,
    pub stream_id: String,
    pub issuer: String,
    pub kind: u8,
    pub reason: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Deserialize)]
struct MoveVecMapEntry {
    key: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct MoveVecMap {
    contents: Vec<MoveVecMapEntry>,
}

#[derive(Debug, Deserialize)]
struct MoveSanction {
    session_id: String,
    stream_id: Address,
    issuer: Address,
    kind: u8,
    reason: String,
    timestamp_ms: u64,
}

#[derive(Debug, Deserialize)]
struct MoveAccount {
    id: Address,
    handle: String,
    owner: Option<Address>,
    protocol: Address,
    sanctions: Vec<MoveSanction>,
    metadata: MoveVecMap,
}

impl From<MoveSanction> for ViewerSanction {
    fn from(s: MoveSanction) -> Self {
        Self {
            session_id: s.session_id,
            stream_id: s.stream_id.to_string(),
            issuer: s.issuer.to_string(),
            kind: s.kind,
            reason: s.reason,
            timestamp_ms: s.timestamp_ms,
        }
    }
}

impl From<MoveAccount> for ViewerAccount {
    fn from(m: MoveAccount) -> Self {
        let sanctions = m.sanctions.into_iter().map(Into::into).collect();
        let metadata = m
            .metadata
            .contents
            .into_iter()
            .map(|e| (e.key, e.value))
            .collect();
        Self {
            id: m.id.to_string(),
            handle: m.handle,
            owner: m.owner.map(|a| a.to_string()),
            protocol: m.protocol.to_string(),
            sanctions,
            metadata,
        }
    }
}

pub async fn account_exists(client: Arc<Client>, account_id: Address) -> Result<bool, SuiError> {
    let obj = read::get_object(client, account_id).await?;
    Ok(obj
        .object_type_opt()
        .is_some_and(|t| t.contains(&format!("{}::viewer::Account", AAVA_PACKAGE))))
}

pub async fn get_account(
    client: Arc<Client>,
    account_id: Address,
) -> Result<ViewerAccount, SuiError> {
    let obj = read::get_object(client, account_id).await?;

    let expected_type = format!("{}::viewer::Account", AAVA_PACKAGE);
    let actual_type = obj.object_type_opt();

    if actual_type.as_deref() != Some(expected_type.as_str()) {
        return Err(SuiError::InvalidInput(format!(
            "Object {account_id} is not a viewer::Account (type={})",
            actual_type.unwrap_or("<missing>")
        )));
    }

    let bytes = obj
        .contents
        .unwrap_or_default()
        .value
        .ok_or_else(|| SuiError::RpcError("RPC object missing `contents`".into()))?;

    let parsed: MoveAccount = bcs::from_bytes(&bytes).map_err(|e| {
        SuiError::ParseError(format!(
            "BCS decode viewer::Account {account_id}: {e} (layout must match Move struct)"
        ))
    })?;

    Ok(parsed.into())
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
