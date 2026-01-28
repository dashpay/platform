use crate::state_transition::StateTransitionFieldTypes;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};

use platform_versioning::PlatformVersioned;

#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

pub mod accessors;
mod fields;
mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
pub mod methods;
mod serialize;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
mod v0;
mod v1;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

pub use fields::*;
use platform_version::version::PlatformVersion;
use platform_version::TryIntoPlatformVersioned;

use crate::data_contract::DataContract;

use crate::identity::state_transition::OptionallyAssetLockProved;
use crate::prelude::IdentityNonce;
pub use v0::*;
pub use v1::*;

pub type DataContractUpdateTransitionLatest = DataContractUpdateTransitionV0;

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
    "dpp.state_transition_serialization_versions.contract_update_state_transition"
)]
pub enum DataContractUpdateTransition {
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(rename = "0"))]
    V0(DataContractUpdateTransitionV0),
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(rename = "1"))]
    V1(DataContractUpdateTransitionV1),
}

impl DataContractUpdateTransition {
    /// Creates a V0 update transition from a full data contract.
    /// This embeds the entire contract in the transition (legacy behavior).
    pub fn from_data_contract_v0(
        data_contract: DataContract,
        identity_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let v0: DataContractUpdateTransitionV0 =
            (data_contract, identity_nonce).try_into_platform_versioned(platform_version)?;
        Ok(v0.into())
    }

    /// Creates a V1 update transition by computing the delta between old and new contracts.
    /// This is the preferred method for V1 transitions as it properly captures the changes.
    pub fn from_contract_update(
        old_contract: &DataContract,
        new_contract: &DataContract,
        identity_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .contract_update_state_transition
            .default_current_version
        {
            0 => {
                Self::from_data_contract_v0(new_contract.clone(), identity_nonce, platform_version)
            }
            1 => {
                let v1 = DataContractUpdateTransitionV1::from_contract_update(
                    old_contract,
                    new_contract,
                    identity_nonce,
                )?;
                Ok(v1.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractUpdateTransition::from_contract_update".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}

impl StateTransitionFieldTypes for DataContractUpdateTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }
}

impl OptionallyAssetLockProved for DataContractUpdateTransition {}

#[cfg(test)]
mod test {
    use crate::data_contract::DataContract;
    use crate::tests::fixtures::get_data_contract_fixture;

    use crate::version::LATEST_PLATFORM_VERSION;

    use platform_version::version::PlatformVersion;

    use super::*;
    use crate::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use crate::state_transition::{StateTransitionLike, StateTransitionOwned, StateTransitionType};

    struct TestData {
        state_transition: DataContractUpdateTransition,
        data_contract: DataContract,
    }

    fn get_test_data() -> TestData {
        let platform_version = PlatformVersion::latest();
        let mut data_contract =
            get_data_contract_fixture(None, 0, platform_version.protocol_version)
                .data_contract_owned();

        // Create a modified version for the update transition
        let old_contract = data_contract.clone();
        data_contract.set_version(2);

        let state_transition = DataContractUpdateTransition::from_contract_update(
            &old_contract,
            &data_contract,
            1,
            platform_version,
        )
        .expect("expected to get transition");

        TestData {
            data_contract,
            state_transition,
        }
    }

    #[test]
    fn should_return_protocol_version() {
        let data = get_test_data();
        assert_eq!(
            LATEST_PLATFORM_VERSION
                .dpp
                .state_transition_serialization_versions
                .contract_update_state_transition
                .default_current_version,
            data.state_transition.state_transition_protocol_version()
        )
    }

    #[test]
    fn should_return_transition_type() {
        let data = get_test_data();
        assert_eq!(
            StateTransitionType::DataContractUpdate,
            data.state_transition.state_transition_type()
        );
    }

    #[test]
    fn should_return_owner_id() {
        let data = get_test_data();
        assert_eq!(
            data.data_contract.owner_id(),
            data.state_transition.owner_id()
        );
    }

    #[test]
    fn is_data_contract_state_transition() {
        let data = get_test_data();
        assert!(data.state_transition.is_data_contract_state_transition());
        assert!(!data.state_transition.is_document_state_transition());
        assert!(!data.state_transition.is_identity_state_transition());
    }
}
