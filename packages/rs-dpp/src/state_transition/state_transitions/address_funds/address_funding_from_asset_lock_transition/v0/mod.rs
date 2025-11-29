#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod proved;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;

use crate::ProtocolError;

use crate::fee::Credits;
use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::identity::KeyOfType;
use crate::prelude::UserFeeIncrease;
use platform_value::BinaryData;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

mod property_names {
    pub const ASSET_LOCK_PROOF: &str = "assetLockProof";
    pub const OUTPUTS: &str = "outputs";
    pub const OUTPUT_PAYING_FEES: &str = "outputPayingFees";
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
    pub outputs: BTreeMap<KeyOfType, Credits>,
    /// The index of the output that will pay fees
    pub output_paying_fees: u16,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}
