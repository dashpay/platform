use anyhow::anyhow;
use lazy_static::lazy_static;
use platform_value::Value;
use serde_json::Value as JsonValue;

use crate::consensus::state::state_error::StateError;

use crate::consensus::state::identity::duplicated_identity_public_key_id_state_error::DuplicatedIdentityPublicKeyIdStateError;
use crate::consensus::state::identity::duplicated_identity_public_key_state_error::DuplicatedIdentityPublicKeyStateError;
use crate::consensus::state::identity::max_identity_public_key_limit_reached_error::MaxIdentityPublicKeyLimitReachedError;
use crate::{
    identity::validation::{duplicated_key_ids, duplicated_keys, TPublicKeysValidator},
    prelude::IdentityPublicKey,
    util::json_value::JsonValueExt,
    validation::SimpleConsensusValidationResult,
    ProtocolError,
};

lazy_static! {
    pub static ref IDENTITY_JSON_SCHEMA: JsonValue =
        serde_json::from_str(include_str!("./../../../schema/identity/identity.json"))
            .expect("Identity Schema file should exist");
    pub static ref IDENTITY_PLATFORM_VALUE_SCHEMA: Value = IDENTITY_JSON_SCHEMA.clone().into();
}

pub struct IdentityUpdatePublicKeysValidator {}
impl TPublicKeysValidator for IdentityUpdatePublicKeysValidator {
    fn validate_keys(
        &self,
        raw_public_keys: &[Value],
    ) -> Result<SimpleConsensusValidationResult, crate::NonConsensusError> {
        validate_public_keys(raw_public_keys)
            .map_err(|e| crate::NonConsensusError::SerdeJsonError(e.to_string()))
    }
}

pub fn validate_public_keys(
    raw_public_keys: &[Value],
) -> Result<SimpleConsensusValidationResult, ProtocolError> {
    let mut validation_result = SimpleConsensusValidationResult::default();

    let maybe_max_items = IDENTITY_JSON_SCHEMA.get_value("properties.publicKeys.maxItems")?;
    let max_items = maybe_max_items
        .as_u64()
        .ok_or_else(|| anyhow!("the maxItems property isn't a integer"))?
        as usize;

    if raw_public_keys.len() > max_items {
        validation_result.add_error(StateError::MaxIdentityPublicKeyLimitReachedError(
            MaxIdentityPublicKeyLimitReachedError::new(max_items),
        ));
        return Ok(validation_result);
    }

    let public_keys: Vec<IdentityPublicKey> = raw_public_keys
        .iter()
        .cloned()
        .map(platform_value::from_value)
        .collect::<Result<_, _>>()?;

    // Check that there's not duplicates key ids in the state transition
    let duplicated_ids = duplicated_key_ids(&public_keys);
    if !duplicated_ids.is_empty() {
        validation_result.add_error(StateError::DuplicatedIdentityPublicKeyIdStateError(
            DuplicatedIdentityPublicKeyIdStateError::new(duplicated_ids),
        ));
        return Ok(validation_result);
    }

    // Check that there's no duplicated keys
    let duplicated_key_ids = duplicated_keys(&public_keys);
    if !duplicated_key_ids.is_empty() {
        validation_result.add_error(StateError::DuplicatedIdentityPublicKeyStateError(
            DuplicatedIdentityPublicKeyStateError::new(duplicated_key_ids),
        ));
    }

    Ok(validation_result)
}
