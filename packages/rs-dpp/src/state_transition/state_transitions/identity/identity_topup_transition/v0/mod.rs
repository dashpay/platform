#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use crate::serialization::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};
use platform_serialization_derive::PlatformSignable;
use std::convert::{TryFrom, TryInto};

use platform_value::{BinaryData, Value};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::identity::Identity;
use crate::identity::KeyType::ECDSA_HASH160;
use crate::prelude::Identifier;

use crate::serialization::PlatformSerializable;
use crate::state_transition::{
    StateTransitionFieldTypes, StateTransitionLike, StateTransitionType,
};
use crate::util::deserializer::ProtocolVersion;
use crate::version::{FeatureVersion, LATEST_VERSION};
use crate::{BlsModule, NonConsensusError, ProtocolError};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

mod property_names {
    pub const ASSET_LOCK_PROOF: &str = "assetLockProof";
    pub const SIGNATURE: &str = "signature";
    pub const PROTOCOL_VERSION: &str = "protocolVersion";
    pub const TRANSITION_TYPE: &str = "type";
    pub const IDENTITY_ID: &str = "identityId";
}

#[derive(Debug, Clone, PlatformDeserialize, PlatformSerialize, PlatformSignable, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]

pub struct IdentityTopUpTransitionV0 {
    // Own ST fields
    pub asset_lock_proof: AssetLockProof,
    pub identity_id: Identifier,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

impl Default for IdentityTopUpTransitionV0 {
    fn default() -> Self {
        Self {
            asset_lock_proof: Default::default(),
            identity_id: Default::default(),
            signature: Default::default(),
        }
    }
}
