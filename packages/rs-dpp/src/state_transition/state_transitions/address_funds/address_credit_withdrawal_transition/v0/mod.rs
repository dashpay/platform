mod state_transition_like;
mod state_transition_validation;
mod types;
pub(super) mod v0_methods;
mod version;

use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::address_funds::{AddressFundsFeeStrategy, AddressWitness, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::{AddressNonce, UserFeeIncrease};
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use crate::{identity::core_script::CoreScript, withdrawal::Pooling, ProtocolError};

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, Encode, Decode, PlatformSignable, PartialEq)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Default)]
pub struct AddressCreditWithdrawalTransitionV0 {
    #[cfg_attr(
        feature = "json-conversion",
        serde(with = "crate::address_funds::serde_helpers::address_input_map")
    )]
    pub inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// Optional output for change
    #[cfg_attr(
        feature = "json-conversion",
        serde(with = "crate::address_funds::serde_helpers::address_output_singular")
    )]
    pub output: Option<(PlatformAddress, Credits)>,
    pub fee_strategy: AddressFundsFeeStrategy,
    pub core_fee_per_byte: u32,
    #[cfg_attr(
        feature = "serde-conversion",
        serde(with = "crate::withdrawal::pooling_serde")
    )]
    pub pooling: Pooling,
    pub output_script: CoreScript,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub input_witnesses: Vec<AddressWitness>,
}
