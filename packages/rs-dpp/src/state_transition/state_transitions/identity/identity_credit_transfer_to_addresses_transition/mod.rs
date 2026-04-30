pub mod accessors;
pub mod fields;
mod identity_signed;
#[cfg(feature = "json-conversion")]
mod json_conversion;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
#[cfg(feature = "value-conversion")]
mod value_conversion;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::state_transition::identity_credit_transfer_to_addresses_transition::fields::property_names::RECIPIENT_ID;
use crate::state_transition::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;
use crate::state_transition::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

use crate::identity::state_transition::OptionallyAssetLockProved;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use fields::*;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

pub type IdentityCreditTransferToAddressesTransitionLatest =
    IdentityCreditTransferToAddressesTransitionV0;

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
    "dpp.state_transition_serialization_versions.identity_credit_transfer_to_addresses_state_transition"
)]
pub enum IdentityCreditTransferToAddressesTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(IdentityCreditTransferToAddressesTransitionV0),
}

impl IdentityCreditTransferToAddressesTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(IdentityCreditTransferToAddressesTransition::V0(
                IdentityCreditTransferToAddressesTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityCreditTransferToAddressesTransitionV0::default_versioned"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl JsonConvertible for IdentityCreditTransferToAddressesTransition {}

impl OptionallyAssetLockProved for IdentityCreditTransferToAddressesTransition {}

impl StateTransitionFieldTypes for IdentityCreditTransferToAddressesTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID, RECIPIENT_ID]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }
}

#[cfg(all(test, feature = "json-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;

    fn fixture() -> IdentityCreditTransferToAddressesTransition {
        IdentityCreditTransferToAddressesTransition::V0(
            IdentityCreditTransferToAddressesTransitionV0::default(),
        )
    }

    #[test]
    fn json_round_trip() {
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered =
            IdentityCreditTransferToAddressesTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_preserves_format_version_tag() {
        let json = fixture().to_json().expect("to_json");
        assert_eq!(json["$formatVersion"], "0");
    }
}
