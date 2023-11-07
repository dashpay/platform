#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod proved;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;

use platform_value::BinaryData;

use serde::{Deserialize, Serialize};

use crate::identity::state_transition::asset_lock_proof::AssetLockProof;

use crate::prelude::Identifier;

use crate::ProtocolError;

mod property_names {
    pub const ASSET_LOCK_PROOF: &str = "assetLockProof";
    pub const SIGNATURE: &str = "signature";
    pub const PROTOCOL_VERSION: &str = "protocolVersion";
    pub const TRANSITION_TYPE: &str = "type";
    pub const IDENTITY_ID: &str = "identityId";
}

#[derive(Debug, Clone, Encode, Decode, PlatformSignable, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Default)]
pub struct IdentityTopUpTransitionV0 {
    // Own ST fields
    pub asset_lock_proof: AssetLockProof,
    pub identity_id: Identifier,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}
