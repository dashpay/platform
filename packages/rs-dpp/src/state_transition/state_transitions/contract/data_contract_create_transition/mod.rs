pub mod accessors;
mod fields;
mod identity_signed;
#[cfg(feature = "json-conversion")]
mod json_conversion;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
mod v0;
#[cfg(feature = "value-conversion")]
mod value_conversion;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use fields::*;

use crate::data_contract::DataContract;
use crate::state_transition::{StateTransition, StateTransitionFieldTypes};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};

use platform_version::{TryFromPlatformVersioned, TryIntoPlatformVersioned};
use platform_versioning::PlatformVersioned;

#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::data_contract::created_data_contract::CreatedDataContract;
use crate::identity::state_transition::OptionallyAssetLockProved;
pub use v0::*;

pub type DataContractCreateTransitionLatest = DataContractCreateTransitionV0;

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
    "dpp.state_transition_serialization_versions.contract_create_state_transition"
)]
pub enum DataContractCreateTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(DataContractCreateTransitionV0),
}

impl TryFromPlatformVersioned<CreatedDataContract> for DataContractCreateTransition {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: CreatedDataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .contract_create_state_transition
            .default_current_version
        {
            0 => {
                let data_contract_create_transition: DataContractCreateTransitionV0 =
                    value.try_into_platform_versioned(platform_version)?;
                Ok(data_contract_create_transition.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractCreateTransition::try_from(CreatedDataContract)".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl TryFromPlatformVersioned<CreatedDataContract> for StateTransition {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: CreatedDataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        let data_contract_create_transition =
            DataContractCreateTransition::try_from_platform_versioned(value, platform_version)?;
        Ok(data_contract_create_transition.into())
    }
}

impl TryFromPlatformVersioned<DataContract> for DataContractCreateTransition {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .contract_create_state_transition
            .default_current_version
        {
            0 => {
                let data_contract_create_transition: DataContractCreateTransitionV0 =
                    value.try_into_platform_versioned(platform_version)?;
                Ok(data_contract_create_transition.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractCreateTransition::try_from(DataContract)".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl StateTransitionFieldTypes for DataContractCreateTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, IDENTITY_NONCE]
    }
}

impl DataContractCreateTransition {
    pub fn state_transition_version(&self) -> u16 {
        match self {
            DataContractCreateTransition::V0(_) => 0,
        }
    }
}

impl OptionallyAssetLockProved for DataContractCreateTransition {}

#[cfg(test)]
mod test {
    use crate::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
    use crate::data_contract::created_data_contract::CreatedDataContract;

    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use crate::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
    use crate::state_transition::traits::StateTransitionLike;
    use crate::state_transition::{
        StateTransitionOwned, StateTransitionType, StateTransitionValueConvert,
    };
    use crate::tests::fixtures::get_data_contract_fixture;

    use crate::version::LATEST_PLATFORM_VERSION;
    use platform_value::Value;

    pub(crate) struct TestData {
        pub(crate) state_transition: DataContractCreateTransition,
        pub(crate) created_data_contract: CreatedDataContract,
    }

    pub(crate) fn get_test_data() -> TestData {
        let created_data_contract = get_data_contract_fixture(None, 0, 1);

        let state_transition =
            <DataContractCreateTransition as StateTransitionValueConvert>::from_object(
                Value::from([
                    (STATE_TRANSITION_PROTOCOL_VERSION, Value::U16(0)),
                    (
                        IDENTITY_NONCE,
                        Value::U64(created_data_contract.identity_nonce()),
                    ),
                    (
                        DATA_CONTRACT,
                        created_data_contract
                            .data_contract()
                            .to_value(LATEST_PLATFORM_VERSION)
                            .unwrap(),
                    ),
                ]),
                LATEST_PLATFORM_VERSION,
            )
            .expect("state transition should be created without errors");

        TestData {
            created_data_contract,
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
                .contract_create_state_transition
                .default_current_version,
            data.state_transition.state_transition_version()
        )
    }

    #[test]
    fn should_return_transition_type() {
        let data = get_test_data();
        assert_eq!(
            StateTransitionType::DataContractCreate,
            data.state_transition.state_transition_type()
        );
    }

    #[test]
    fn should_return_data_contract() {
        let data = get_test_data();

        let data_contract = DataContract::try_from_platform_versioned(
            data.state_transition.data_contract().clone(),
            false,
            &mut vec![],
            LATEST_PLATFORM_VERSION,
        )
        .expect("to get data contract");

        assert_eq!(
            data_contract
                .to_json(LATEST_PLATFORM_VERSION)
                .expect("conversion to object shouldn't fail"),
            data.created_data_contract
                .data_contract()
                .to_json(LATEST_PLATFORM_VERSION)
                .expect("conversion to object shouldn't fail")
        );
    }

    #[test]
    fn should_return_owner_id() {
        let data = get_test_data();
        assert_eq!(
            data.created_data_contract.data_contract().owner_id(),
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
    fn should_roundtrip_via_from_object() {
        let data = get_test_data();

        // Convert to object and back
        let mut obj = StateTransitionValueConvert::to_object(&data.state_transition, false)
            .expect("to_object should succeed");

        // Add the protocol version field for from_object
        obj.insert(STATE_TRANSITION_PROTOCOL_VERSION.to_string(), Value::U16(0))
            .expect("insert should succeed");

        let restored = <DataContractCreateTransition as StateTransitionValueConvert>::from_object(
            obj,
            LATEST_PLATFORM_VERSION,
        )
        .expect("from_object should succeed");

        assert_eq!(data.state_transition, restored);
    }

    #[test]
    fn should_roundtrip_via_from_value_map() {
        let data = get_test_data();

        let obj = StateTransitionValueConvert::to_object(&data.state_transition, false)
            .expect("to_object should succeed");

        let mut map = obj
            .into_btree_string_map()
            .expect("should convert to btree map");
        map.insert(STATE_TRANSITION_PROTOCOL_VERSION.to_string(), Value::U16(0));

        let restored =
            <DataContractCreateTransition as StateTransitionValueConvert>::from_value_map(
                map,
                LATEST_PLATFORM_VERSION,
            )
            .expect("from_value_map should succeed");

        assert_eq!(data.state_transition, restored);
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

        // With sufficient balance, validation should pass
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

        // With zero balance, validation should fail
        let result = data
            .state_transition
            .validate_estimated_fee(0, LATEST_PLATFORM_VERSION)
            .expect("validation should succeed");
        assert!(!result.is_valid());
    }

    #[test]
    fn should_create_from_created_data_contract() {
        let created_data_contract = get_data_contract_fixture(None, 0, 1);

        let transition = DataContractCreateTransition::try_from_platform_versioned(
            created_data_contract.clone(),
            LATEST_PLATFORM_VERSION,
        )
        .expect("should create transition from created data contract");

        assert_eq!(
            transition.identity_nonce(),
            created_data_contract.identity_nonce()
        );
    }

    #[test]
    fn should_create_state_transition_from_created_data_contract() {
        let created_data_contract = get_data_contract_fixture(None, 0, 1);

        let st = StateTransition::try_from_platform_versioned(
            created_data_contract,
            LATEST_PLATFORM_VERSION,
        )
        .expect("should create state transition from created data contract");

        match st {
            StateTransition::DataContractCreate(_) => {}
            _ => panic!("expected DataContractCreate"),
        }
    }

    #[test]
    fn v0_should_roundtrip_via_from_object() {
        let data = get_test_data();
        match &data.state_transition {
            DataContractCreateTransition::V0(v0) => {
                let obj = v0.to_object(false).expect("to_object should succeed");

                let restored =
                    DataContractCreateTransitionV0::from_object(obj, LATEST_PLATFORM_VERSION)
                        .expect("from_object should succeed");

                assert_eq!(*v0, restored);
            }
        }
    }

    #[test]
    fn v0_should_roundtrip_via_from_value_map() {
        let data = get_test_data();
        match &data.state_transition {
            DataContractCreateTransition::V0(v0) => {
                let obj = v0.to_object(false).expect("to_object should succeed");
                let map = obj
                    .into_btree_string_map()
                    .expect("should convert to btree map");

                let restored =
                    DataContractCreateTransitionV0::from_value_map(map, LATEST_PLATFORM_VERSION)
                        .expect("from_value_map should succeed");

                assert_eq!(*v0, restored);
            }
        }
    }

    #[test]
    fn v0_should_create_from_created_data_contract() {
        let created_data_contract = get_data_contract_fixture(None, 5, 1);

        let v0 = DataContractCreateTransitionV0::try_from_platform_versioned(
            created_data_contract,
            LATEST_PLATFORM_VERSION,
        )
        .expect("should create v0 from created data contract");

        assert_eq!(v0.identity_nonce, 5);
        assert_eq!(v0.user_fee_increase, 0);
    }
}
