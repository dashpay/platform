#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::state_transition::StateTransitionFieldTypes;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};

use platform_versioning::PlatformVersioned;

#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

pub mod accessors;
mod fields;
mod identity_signed;
#[cfg(feature = "json-conversion")]
mod json_conversion;
pub mod methods;
mod serialize;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
mod v0;
#[cfg(feature = "value-conversion")]
mod value_conversion;
mod version;

pub use fields::*;
use platform_version::version::PlatformVersion;
use platform_version::{TryFromPlatformVersioned, TryIntoPlatformVersioned};

use crate::data_contract::DataContract;

use crate::identity::state_transition::OptionallyAssetLockProved;
use crate::prelude::IdentityNonce;
pub use v0::*;

pub type DataContractUpdateTransitionLatest = DataContractUpdateTransitionV0;

#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
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
    "dpp.state_transition_serialization_versions.contract_update_state_transition"
)]
pub enum DataContractUpdateTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(DataContractUpdateTransitionV0),
}

impl TryFromPlatformVersioned<(DataContract, IdentityNonce)> for DataContractUpdateTransition {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: (DataContract, IdentityNonce),
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
    use crate::state_transition::{StateTransitionLike, StateTransitionOwned, StateTransitionType};

    struct TestData {
        state_transition: DataContractUpdateTransition,
        data_contract: DataContract,
    }

    fn get_test_data() -> TestData {
        let platform_version = PlatformVersion::first();
        let data_contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();

        let state_transition: DataContractUpdateTransition = (data_contract.clone(), 1)
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
    #[cfg(feature = "json-conversion")]
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

    #[test]
    fn should_validate_estimated_fee_with_sufficient_balance() {
        use crate::state_transition::StateTransitionEstimatedFeeValidation;
        use crate::state_transition::StateTransitionIdentityEstimatedFeeValidation;

        let data = get_test_data();
        let fee = data
            .state_transition
            .calculate_min_required_fee(LATEST_PLATFORM_VERSION)
            .expect("fee calculation should succeed");

        let result = data
            .state_transition
            .validate_estimated_fee(fee + 1000, LATEST_PLATFORM_VERSION)
            .expect("validation should succeed");
        assert!(result.is_valid());
    }

    #[test]
    fn should_validate_estimated_fee_with_insufficient_balance() {
        use crate::state_transition::StateTransitionIdentityEstimatedFeeValidation;

        let data = get_test_data();

        let result = data
            .state_transition
            .validate_estimated_fee(0, LATEST_PLATFORM_VERSION)
            .expect("validation should succeed");
        assert!(!result.is_valid());
    }

    #[test]
    #[cfg(feature = "json-conversion")]
    fn should_convert_to_json() {
        use crate::state_transition::JsonStateTransitionSerializationOptions;
        use crate::state_transition::StateTransitionJsonConvert;

        let data = get_test_data();
        let json = <DataContractUpdateTransition as StateTransitionJsonConvert>::to_json(
            &data.state_transition,
            JsonStateTransitionSerializationOptions {
                skip_signature: false,
                into_validating_json: false,
            },
        )
        .expect("to_json should succeed");

        assert!(json.is_object());

        // Verify protocol version is set
        let version = json
            .get(STATE_TRANSITION_PROTOCOL_VERSION)
            .expect("should have version");
        assert_eq!(version.as_u64(), Some(0));
    }

    #[test]
    #[cfg(feature = "value-conversion")]
    fn should_deserialize_from_object() {
        use crate::state_transition::state_transitions::common_fields::property_names::IDENTITY_CONTRACT_NONCE;
        use crate::state_transition::StateTransitionValueConvert;

        let data = get_test_data();

        let mut obj = StateTransitionValueConvert::to_object(&data.state_transition, false)
            .expect("to_object should succeed");

        // The serde field name for identity_contract_nonce is "$identity-contract-nonce"
        // but from_object expects "identityContractNonce". Fix up the field name.
        if let Ok(nonce) = obj.remove("$identity-contract-nonce") {
            obj.insert(IDENTITY_CONTRACT_NONCE.to_string(), nonce)
                .expect("insert should succeed");
        }

        let restored = <DataContractUpdateTransition as StateTransitionValueConvert>::from_object(
            obj,
            PlatformVersion::first(),
        )
        .expect("from_object should succeed");

        assert_eq!(data.state_transition, restored);
    }

    #[test]
    #[cfg(feature = "value-conversion")]
    fn should_deserialize_from_value_map() {
        use crate::state_transition::state_transitions::common_fields::property_names::IDENTITY_CONTRACT_NONCE;
        use crate::state_transition::StateTransitionValueConvert;

        let data = get_test_data();

        let obj = StateTransitionValueConvert::to_object(&data.state_transition, false)
            .expect("to_object should succeed");
        let mut map = obj
            .into_btree_string_map()
            .expect("should convert to btree map");

        // Fix up the serde-renamed field to the expected key for from_value_map
        if let Some(nonce) = map.remove("$identity-contract-nonce") {
            map.insert(IDENTITY_CONTRACT_NONCE.to_string(), nonce);
        }

        let restored =
            <DataContractUpdateTransition as StateTransitionValueConvert>::from_value_map(
                map,
                PlatformVersion::first(),
            )
            .expect("from_value_map should succeed");

        assert_eq!(data.state_transition, restored);
    }

    #[test]
    #[cfg(feature = "value-conversion")]
    fn v0_should_deserialize_from_object() {
        use crate::state_transition::state_transitions::common_fields::property_names::IDENTITY_CONTRACT_NONCE;
        use crate::state_transition::StateTransitionValueConvert;

        let data = get_test_data();
        match &data.state_transition {
            DataContractUpdateTransition::V0(v0) => {
                let mut obj = StateTransitionValueConvert::to_object(v0, false)
                    .expect("to_object should succeed");

                // Fix up the serde-renamed field
                if let Ok(nonce) = obj.remove("$identity-contract-nonce") {
                    obj.insert(IDENTITY_CONTRACT_NONCE.to_string(), nonce)
                        .expect("insert should succeed");
                }

                let restored =
                    DataContractUpdateTransitionV0::from_object(obj, PlatformVersion::first())
                        .expect("from_object should succeed");

                assert_eq!(*v0, restored);
            }
        }
    }

    #[test]
    #[cfg(feature = "value-conversion")]
    fn v0_should_deserialize_from_value_map() {
        use crate::state_transition::state_transitions::common_fields::property_names::IDENTITY_CONTRACT_NONCE;
        use crate::state_transition::StateTransitionValueConvert;

        let data = get_test_data();
        match &data.state_transition {
            DataContractUpdateTransition::V0(v0) => {
                let obj = StateTransitionValueConvert::to_object(v0, false)
                    .expect("to_object should succeed");
                let mut map = obj
                    .into_btree_string_map()
                    .expect("should convert to btree map");

                // Fix up the serde-renamed field
                if let Some(nonce) = map.remove("$identity-contract-nonce") {
                    map.insert(IDENTITY_CONTRACT_NONCE.to_string(), nonce);
                }

                let restored =
                    DataContractUpdateTransitionV0::from_value_map(map, PlatformVersion::first())
                        .expect("from_value_map should succeed");

                assert_eq!(*v0, restored);
            }
        }
    }
}
