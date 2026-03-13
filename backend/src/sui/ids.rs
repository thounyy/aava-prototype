use std::str::FromStr;

use bcs;
use sui_sdk_types::{Address, TypeTag};

use crate::sui::constants::ACCOUNT_REGISTRY;
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
