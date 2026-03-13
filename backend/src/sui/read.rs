use std::sync::Arc;
use std::str::FromStr;

use bcs;
use prost_types::value::Kind;
use sui_rpc::field::{FieldMask, FieldMaskUtil};
use sui_rpc::proto::sui::rpc::v2::ListDynamicFieldsRequest;
use sui_rpc::Client;
use sui_sdk_types::{Address, TypeTag};

use crate::sui::constants::{ACCOUNT_REGISTRY, WALRUS_SYSTEM};
use crate::sui::error::SuiError;

/// Derive the Account object ID from a deterministic client identifier.
///
/// This mirrors `derived_object::claim(registry.uid_mut(), username)` in Move.
pub fn derive_account_id(account_identifier: &str) -> Result<Address, SuiError> {
    let parent: Address = ACCOUNT_REGISTRY
        .parse()
        .map_err(|e| SuiError::ParseError(format!("Invalid ACCOUNT_REGISTRY constant: {e}")))?;

    // `new_account_for_testing` uses `std::string::String`.
    let key_type_tag = TypeTag::from_str("0x1::string::String")
        .map_err(|e| SuiError::ParseError(format!("Invalid key type tag: {e}")))?;
    let key_bytes = bcs::to_bytes(&account_identifier.to_string())
        .map_err(|e| SuiError::ParseError(format!("Failed to BCS-encode account key: {e}")))?;

    Ok(parent.derive_object_id(&key_type_tag, &key_bytes))
}


pub struct WalrusSystemParams {
    pub price_per_unit_size: u64,
    pub n_shards: u16,
}

pub async fn fetch_walrus_system_params(
    client: Arc<Client>,
) -> Result<WalrusSystemParams, SuiError> {
    let parent_id: Address = WALRUS_SYSTEM
        .parse()
        .map_err(|e| SuiError::ParseError(format!("Invalid WALRUS_SYSTEM constant: {e}")))?;

    let resp = client
        .as_ref()
        .clone()
        .state_client()
        .list_dynamic_fields(
            ListDynamicFieldsRequest::default()
                .with_parent(parent_id)
                .with_read_mask(FieldMask::from_paths([
                    "name",
                    "value_type",
                    "field_object.json",
                    "field_object.object_type",
                ])),
        )
        .await
        .map_err(|e| SuiError::RpcError(format!("Failed to list Walrus system dynamic fields: {e}")))?
        .into_inner();

    if resp.dynamic_fields.len() != 1 {
        return Err(SuiError::ParseError(
            "Unexpected number of dynamic fields under Walrus System".into(),
        ));
    }

    let df = &resp.dynamic_fields[0];
    let is_inner = df
        .value_type_opt()
        .map(|t| t.contains("system_state_inner::SystemStateInnerV1"))
        .unwrap_or(false);
    if !is_inner {
        return Err(SuiError::ParseError(format!(
            "Expected SystemStateInnerV1, got {:?}",
            df.value_type_opt()
        )));
    }

    let field_obj = df
        .field_object_opt()
        .ok_or_else(|| SuiError::ParseError("Missing field_object for SystemStateInnerV1".into()))?;

    let fields = match field_obj.json_opt().and_then(|v| v.kind.as_ref()) {
        Some(Kind::StructValue(s)) => &s.fields,
        _ => return Err(SuiError::ParseError("Missing JSON on SystemStateInnerV1".into())),
    };

    let inner = match fields.get("value").and_then(|v| v.kind.as_ref()) {
        Some(Kind::StructValue(s)) => &s.fields,
        _ => return Err(SuiError::ParseError("Missing 'value' in SystemStateInnerV1".into())),
    };

    let parse_u64 = |key: &str| -> Result<u64, SuiError> {
        match inner.get(key).and_then(|v| v.kind.as_ref()) {
            Some(Kind::StringValue(s)) => s
                .parse()
                .map_err(|_| SuiError::ParseError(format!("{key} is not a valid u64"))),
            _ => Err(SuiError::ParseError(format!("Missing {key} in SystemStateInnerV1"))),
        }
    };

    let price_per_unit_size =
        parse_u64("storage_price_per_unit_size")? + parse_u64("write_price_per_unit_size")?;

    let committee = match inner.get("committee").and_then(|v| v.kind.as_ref()) {
        Some(Kind::StructValue(s)) => &s.fields,
        _ => return Err(SuiError::ParseError("Missing 'committee' in SystemStateInnerV1".into())),
    };

    let n_shards = match committee.get("n_shards").and_then(|v| v.kind.as_ref()) {
        Some(Kind::NumberValue(n)) => {
            let v = *n as u16;
            if v == 0 {
                return Err(SuiError::ParseError("n_shards is zero".into()));
            }
            v
        }
        _ => return Err(SuiError::ParseError("Missing n_shards in committee".into())),
    };

    Ok(WalrusSystemParams {
        price_per_unit_size,
        n_shards,
    })
}
