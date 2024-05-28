pub mod accessors;
pub mod fields;
mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
pub mod methods;
mod state_transition_like;
pub mod v0;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use crate::state_transition::masternode_vote_transition::fields::property_names::PRO_TX_HASH;
use crate::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
use crate::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

use crate::identity::state_transition::OptionallyAssetLockProved;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use fields::*;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

pub type MasternodeVoteTransitionLatest = MasternodeVoteTransitionV0;

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformSignable,
    PlatformVersioned,
    From,
    PartialEq,
)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$version")
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path_bounds(
    "dpp.state_transition_serialization_versions.masternode_vote_state_transition"
)]
pub enum MasternodeVoteTransition {
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(rename = "0"))]
    V0(MasternodeVoteTransitionV0),
}

impl MasternodeVoteTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(MasternodeVoteTransition::V0(
                MasternodeVoteTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "MasternodeVoteTransitionV0::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl OptionallyAssetLockProved for MasternodeVoteTransition {}

impl StateTransitionFieldTypes for MasternodeVoteTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![PRO_TX_HASH]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }
}
