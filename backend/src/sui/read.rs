use std::sync::Arc;

use axum::http::StatusCode;
use sui_rpc::field::{FieldMask, FieldMaskUtil};
use sui_rpc::proto::sui::rpc::v2::ListDynamicFieldsRequest;
use sui_rpc::Client;
use sui_sdk_types::Address;

use crate::sui::constants::WALRUS_SYSTEM;

fn parse_u64(value: &serde_json::Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<u64>().ok()))
}

fn prost_value_to_json(value: &prost_types::Value) -> serde_json::Value {
    use prost_types::value::Kind;
    match value.kind.as_ref() {
        Some(Kind::NullValue(_)) | None => serde_json::Value::Null,
        Some(Kind::NumberValue(n)) => serde_json::json!(n),
        Some(Kind::StringValue(s)) => serde_json::Value::String(s.clone()),
        Some(Kind::BoolValue(b)) => serde_json::Value::Bool(*b),
        Some(Kind::StructValue(s)) => {
            let mut map = serde_json::Map::with_capacity(s.fields.len());
            for (k, v) in &s.fields {
                map.insert(k.clone(), prost_value_to_json(v));
            }
            serde_json::Value::Object(map)
        }
        Some(Kind::ListValue(l)) => {
            serde_json::Value::Array(l.values.iter().map(prost_value_to_json).collect())
        }
    }
}

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
    let json = field_obj.json_opt().ok_or_else(|| {
        (
            StatusCode::BAD_GATEWAY,
            "Missing JSON on SystemStateInnerV1 field object".to_string(),
        )
    })?;

    // TODO: simplify this once we know the JSON structure
    let json_value = prost_value_to_json(json);

    let storage = json_value
        .pointer("/value/storage_price_per_unit_size")
        .and_then(parse_u64)
        .or_else(|| {
            json_value
                .pointer("/storage_price_per_unit_size")
                .and_then(parse_u64)
        });
    let write = json_value
        .pointer("/value/write_price_per_unit_size")
        .and_then(parse_u64)
        .or_else(|| {
            json_value
                .pointer("/write_price_per_unit_size")
                .and_then(parse_u64)
        });

    if let (Some(storage_price_per_unit_size), Some(write_price_per_unit_size)) = (storage, write) {
        Ok(storage_price_per_unit_size + write_price_per_unit_size)
    } else {
        Err((
            StatusCode::BAD_GATEWAY,
            "SystemStateInnerV1 JSON is missing price fields".to_string(),
        ))
    }
}
