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
use std::collections::BTreeMap;

use crate::address_funds::AddressWitness;
use crate::fee::Credits;
use crate::identity::KeyOfType;
use crate::prelude::{Identifier, KeyOfTypeNonce, UserFeeIncrease};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::ProtocolError;

#[derive(Debug, Clone, Encode, Decode, PlatformSignable, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Default)]
pub struct IdentityTopUpFromAddressesTransitionV0 {
    // Own ST fields
    pub inputs: BTreeMap<KeyOfType, (KeyOfTypeNonce, Credits)>,
    pub outputs: BTreeMap<KeyOfType, Credits>,
    pub identity_id: Identifier,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub input_witnesses: Vec<AddressWitness>,
}
