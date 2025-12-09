#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod proved;
mod state_transition_like;
mod state_transition_validation;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;

use crate::ProtocolError;

use crate::address_funds::{AddressFundsFeeStrategy, AddressWitness, PlatformAddress};
use crate::fee::Credits;
use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::prelude::{AddressNonce, UserFeeIncrease};
use platform_value::BinaryData;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

mod property_names {
    pub const ASSET_LOCK_PROOF: &str = "assetLockProof";
    pub const INPUTS: &str = "inputs";
    pub const OUTPUTS: &str = "outputs";
    pub const FEE_STRATEGY: &str = "feeStrategy";
    pub const SIGNATURE: &str = "signature";
    pub const PROTOCOL_VERSION: &str = "protocolVersion";
    pub const TRANSITION_TYPE: &str = "type";
}

#[derive(Debug, Clone, PartialEq, Encode, Decode, PlatformSignable)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Default)]
pub struct AddressFundingFromAssetLockTransitionV0 {
    pub asset_lock_proof: AssetLockProof,
    /// Inputs from existing platform addresses (optional, for combining funds)
    pub inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// Outputs to fund platform addresses.
    /// - `Some(credits)` = explicit amount to send to this address
    /// - `None` = this address receives everything remaining after explicit outputs and fees
    /// Exactly one output must be `None` to receive the remainder (ensures full asset lock consumption).
    pub outputs: BTreeMap<PlatformAddress, Option<Credits>>,
    pub fee_strategy: AddressFundsFeeStrategy,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
    #[platform_signable(exclude_from_sig_hash)]
    pub input_witnesses: Vec<AddressWitness>,
}
