use std::sync::Arc;

use axum::http::StatusCode;
use prost_types::value::Kind;
use sui_rpc::field::{FieldMask, FieldMaskUtil};
use sui_rpc::proto::sui::rpc::v2::ListDynamicFieldsRequest;
use sui_rpc::Client;
use sui_sdk_types::Address;

use crate::sui::constants::WALRUS_SYSTEM;

pub async fn fetch_walrus_price_per_unit_size(
    client: Arc<Client>,
) -> Result<u64, (StatusCode, String)> {
    let parent_id: Address = WALRUS_SYSTEM.parse().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid WALRUS_SYSTEM constant: {e}"),
        )
    })?;

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
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to list Walrus system dynamic fields: {e}"),
            )
        })?
        .into_inner();

    if resp.dynamic_fields.len() != 1 {
        return Err((
            StatusCode::BAD_GATEWAY,
            "Unexpected number of dynamic fields under Walrus System".to_string(),
        ));
    }

    let df = &resp.dynamic_fields[0];
    let is_inner = df
        .value_type_opt()
        .map(|t| t.contains("system_state_inner::SystemStateInnerV1"))
        .unwrap_or(false);
    if !is_inner {
        return Err((
            StatusCode::BAD_GATEWAY,
            format!(
                "Unexpected dynamic field type, expected SystemStateInnerV1, got {:?}",
                df.value_type_opt()
            ),
        ));
    }

    let field_obj = df.field_object_opt().ok_or_else(|| {
        (
            StatusCode::BAD_GATEWAY,
            "Missing field_object for SystemStateInnerV1 dynamic field".to_string(),
        )
    })?;
    let fields = match field_obj.json_opt().and_then(|v| v.kind.as_ref()) {
        Some(Kind::StructValue(s)) => &s.fields,
        _ => {
            return Err((
                StatusCode::BAD_GATEWAY,
                "Missing JSON on SystemStateInnerV1 field object".to_string(),
            ))
        }
    };
    let inner = match fields.get("value").and_then(|v| v.kind.as_ref()) {
        Some(Kind::StructValue(s)) => &s.fields,
        _ => {
            return Err((
                StatusCode::BAD_GATEWAY,
                "Missing 'value' in SystemStateInnerV1".to_string(),
            ))
        }
    };

    let parse = |key: &str| -> Result<u64, (StatusCode, String)> {
        match inner.get(key).and_then(|v| v.kind.as_ref()) {
            Some(Kind::StringValue(s)) => s.parse().map_err(|_| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("{key} is not a valid u64"),
                )
            }),
            _ => Err((
                StatusCode::BAD_GATEWAY,
                format!("Missing {key} in SystemStateInnerV1"),
            )),
        }
    };

    Ok(parse("storage_price_per_unit_size")? + parse("write_price_per_unit_size")?)
}
