#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod state_transition_like;
mod state_transition_validation;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;
use std::collections::BTreeMap;

use crate::address_funds::{AddressFundsFeeStrategy, AddressWitness, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::{AddressNonce, Identifier, UserFeeIncrease};
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
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(with = "crate::address_funds::platform_address_map_serde")
    )]
    pub inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// Optional output to send remaining credits to an address
    pub output: Option<(PlatformAddress, Credits)>,
    pub identity_id: Identifier,
    pub fee_strategy: AddressFundsFeeStrategy,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub input_witnesses: Vec<AddressWitness>,
}
