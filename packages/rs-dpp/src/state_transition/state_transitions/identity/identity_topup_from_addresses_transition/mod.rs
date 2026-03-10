pub mod accessors;
pub mod fields;
#[cfg(feature = "json-conversion")]
mod json_conversion;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_fee_strategy;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
#[cfg(feature = "value-conversion")]
mod value_conversion;
mod version;

#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use fields::*;

use crate::state_transition::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
use crate::state_transition::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

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
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path_bounds(
    "dpp.state_transition_serialization_versions.identity_top_up_from_addresses_state_transition"
)]
pub enum IdentityTopUpFromAddressesTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(IdentityTopUpFromAddressesTransitionV0),
}

impl IdentityTopUpFromAddressesTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(IdentityTopUpFromAddressesTransition::V0(
                IdentityTopUpFromAddressesTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityTopUpFromAddressesTransition::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl StateTransitionFieldTypes for IdentityTopUpFromAddressesTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }
}
