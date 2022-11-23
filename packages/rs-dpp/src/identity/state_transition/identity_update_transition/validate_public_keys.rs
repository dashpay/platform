use anyhow::anyhow;
use lazy_static::lazy_static;
use serde_json::Value as JsonValue;

use crate::{
    identity::validation::{duplicated_key_ids, duplicated_keys, TPublicKeysValidator},
    prelude::IdentityPublicKey,
    util::json_value::JsonValueExt,
    validation::SimpleValidationResult,
    ProtocolError, StateError,
};

lazy_static! {
    pub static ref IDENTITY_JSON_SCHEMA: JsonValue =
        serde_json::from_str(include_str!("./../../../schema/identity/identity.json"))
            .expect("Identity Schema file should exist");
}

pub struct IdentityUpdatePublicKeysValidator {}
impl TPublicKeysValidator for IdentityUpdatePublicKeysValidator {
    fn validate_keys(
        &self,
        raw_public_keys: &[JsonValue],
    ) -> Result<SimpleValidationResult, crate::NonConsensusError> {
        validate_public_keys(raw_public_keys)
            .map_err(|e| crate::NonConsensusError::SerdeJsonError(e.to_string()))
    }
}

pub fn validate_public_keys(
    raw_public_keys: &[JsonValue],
) -> Result<SimpleValidationResult, ProtocolError> {
    let mut validation_result = SimpleValidationResult::default();

    let maybe_max_items = IDENTITY_JSON_SCHEMA.get_value("properties.publicKeys.maxItems")?;
    let max_items = maybe_max_items
        .as_u64()
        .ok_or_else(|| anyhow!("the maxItems property isn't a integer"))?
        as usize;

    if raw_public_keys.len() > max_items {
        validation_result.add_error(StateError::MaxIdentityPublicKeyLimitReached { max_items });
        return Ok(validation_result);
    }

    let public_keys: Vec<IdentityPublicKey> = raw_public_keys
        .iter()
        .cloned()
        .map(serde_json::from_value)
        .collect::<Result<_, _>>()?;

    // Check that there's not duplicates key ids in the state transition
    let duplicated_ids = duplicated_key_ids(&public_keys);
    if !duplicated_ids.is_empty() {
        validation_result
            .add_error(StateError::DuplicatedIdentityPublicKeyIdError { duplicated_ids });
    }

    // Check that there's no duplicated keys
    let duplicated_key_ids = duplicated_keys(&public_keys);
    if !duplicated_key_ids.is_empty() {
        validation_result.add_error(StateError::DuplicatedIdentityPublicKeyError {
            duplicated_public_key_ids: duplicated_key_ids,
        });
    }

    Ok(validation_result)
}
