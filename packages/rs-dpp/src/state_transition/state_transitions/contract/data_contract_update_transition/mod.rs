use crate::state_transition::StateTransitionFieldTypes;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};

use platform_versioning::PlatformVersioned;

use serde::{Deserialize, Serialize};

pub mod accessors;
mod fields;
mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
pub mod methods;
mod serialize;
mod state_transition_like;
mod v0;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

pub use fields::*;
use platform_version::version::PlatformVersion;
use platform_version::{TryFromPlatformVersioned, TryIntoPlatformVersioned};

use crate::data_contract::DataContract;

use crate::identity::state_transition::OptionallyAssetLockProved;
pub use v0::*;

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
}

impl TryFromPlatformVersioned<DataContract> for DataContractUpdateTransition {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .contract_update_state_transition
            .default_current_version
        {
            0 => {
                let data_contract_update_transition: DataContractUpdateTransitionV0 =
                    value.try_into_platform_versioned(platform_version)?;
                Ok(data_contract_update_transition.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractUpdateTransition::try_from_platform_versioned(DataContract)"
                    .to_string(),
                known_versions: vec![0],
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
    use crate::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
    use crate::tests::fixtures::get_data_contract_fixture;

    use crate::version::LATEST_PLATFORM_VERSION;

    use platform_version::version::PlatformVersion;

    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::state_transition::{StateTransitionLike, StateTransitionType};

    struct TestData {
        state_transition: DataContractUpdateTransition,
        data_contract: DataContract,
    }

    fn get_test_data() -> TestData {
        let platform_version = PlatformVersion::first();
        let data_contract = get_data_contract_fixture(None, platform_version.protocol_version)
            .data_contract_owned();

        let state_transition: DataContractUpdateTransition = data_contract
            .clone()
            .try_into_platform_versioned(platform_version)
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
    #[cfg(feature = "state-transition-json-conversion")]
    fn should_return_data_contract() {
        let data = get_test_data();

        assert_eq!(
            data.state_transition.data_contract().clone(),
            data.data_contract
                .try_into_platform_versioned(PlatformVersion::first())
                .unwrap()
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
