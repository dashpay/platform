mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;
use platform_value::{BinaryData, Value};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

use std::convert::{TryFrom, TryInto};

use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreationSignable;

use crate::prelude::{IdentityNonce, UserFeeIncrease};
use crate::{
    identity::KeyID,
    prelude::{Identifier, Revision},
    ProtocolError,
};

#[derive(Encode, Decode, PlatformSignable, Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
// There is a problem deriving bincode for a borrowed vector
// Hence we set to do it somewhat manually inside the PlatformSignable proc macro
// Instead of inside of bincode_derive
#[platform_signable(derive_bincode_with_borrowed_vec)]
#[derive(Default)]
pub struct IdentityUpdateTransitionV0 {
    /// Unique identifier of the identity to be updated
    pub identity_id: Identifier,

    /// The revision of the identity after update
    pub revision: Revision,

    /// Identity nonce for this transition to prevent replay attacks
    pub nonce: IdentityNonce,

    /// Public Keys to add to the Identity
    /// we want to skip serialization of transitions, as we does it manually in `to_object()`  and `to_json()`
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(default))]
    #[platform_signable(into = "Vec<IdentityPublicKeyInCreationSignable>")]
    pub add_public_keys: Vec<IdentityPublicKeyInCreation>,

    /// Identity Public Keys ID's to disable for the Identity
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(default))]
    pub disable_public_keys: Vec<KeyID>,

    /// The fee multiplier
    pub user_fee_increase: UserFeeIncrease,

    /// The ID of the public key used to sing the State Transition
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    /// Cryptographic signature of the State Transition
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

/// if the property isn't present the empty list is returned. If property is defined, the function
/// might return some serialization-related errors
fn get_list<T: TryFrom<Value, Error = platform_value::Error>>(
    value: &mut Value,
    property_name: &str,
) -> Result<Vec<T>, ProtocolError> {
    value
        .remove_optional_array(property_name)
        .map_err(ProtocolError::ValueError)?
        .unwrap_or_default()
        .into_iter()
        .map(|value| value.try_into().map_err(ProtocolError::ValueError))
        .collect()
}

/// if the property isn't present the empty list is returned. If property is defined, the function
/// might return some serialization-related errors
fn remove_integer_list_or_default<T>(
    value: &mut Value,
    property_name: &str,
) -> Result<Vec<T>, ProtocolError>
where
    T: TryFrom<i128>
        + TryFrom<u128>
        + TryFrom<u64>
        + TryFrom<i64>
        + TryFrom<u32>
        + TryFrom<i32>
        + TryFrom<u16>
        + TryFrom<i16>
        + TryFrom<u8>
        + TryFrom<i8>,
{
    value
        .remove_optional_array(property_name)
        .map_err(ProtocolError::ValueError)?
        .unwrap_or_default()
        .into_iter()
        .map(|value| value.to_integer().map_err(ProtocolError::ValueError))
        .collect()
}
