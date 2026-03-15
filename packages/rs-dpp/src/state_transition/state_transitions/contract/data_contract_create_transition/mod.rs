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
    fn should_return_modified_data_ids() {
        let data = get_test_data();
        let ids = data.state_transition.modified_data_ids();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], data.created_data_contract.data_contract().id());
    }

    #[test]
    fn should_return_unique_identifiers() {
        let data = get_test_data();
        let unique_ids = data.state_transition.unique_identifiers();
        assert_eq!(unique_ids.len(), 1);
        let expected = format!(
            "dcc-{}-{}",
            data.created_data_contract.data_contract().owner_id(),
            data.created_data_contract.data_contract().id()
        );
        assert_eq!(unique_ids[0], expected);
    }

    #[test]
    fn should_return_state_transition_protocol_version() {
        let data = get_test_data();
        assert_eq!(data.state_transition.state_transition_protocol_version(), 0);
    }

    #[test]
    fn should_handle_identity_signed_methods() {
        use crate::identity::Purpose;
        use crate::identity::SecurityLevel;
        use crate::state_transition::StateTransitionIdentitySigned;

        let data = get_test_data();

        // Test signature_public_key_id
        assert_eq!(data.state_transition.signature_public_key_id(), 0);

        // Test set_signature_public_key_id
        let mut transition = data.state_transition.clone();
        transition.set_signature_public_key_id(42);
        assert_eq!(transition.signature_public_key_id(), 42);

        // Test security_level_requirement
        let levels = data
            .state_transition
            .security_level_requirement(Purpose::AUTHENTICATION);
        assert!(levels.contains(&SecurityLevel::CRITICAL));
        assert!(levels.contains(&SecurityLevel::HIGH));
        assert_eq!(levels.len(), 2);
    }

    #[test]
    fn should_handle_user_fee_increase() {
        use crate::state_transition::StateTransitionHasUserFeeIncrease;

        let data = get_test_data();
        assert_eq!(data.state_transition.user_fee_increase(), 0);

        let mut transition = data.state_transition;
        transition.set_user_fee_increase(5);
        assert_eq!(transition.user_fee_increase(), 5);
    }

    #[test]
    fn should_handle_signature_methods() {
        use crate::state_transition::StateTransitionSingleSigned;

        let data = get_test_data();
        assert!(data.state_transition.signature().is_empty());

        let mut transition = data.state_transition;
        transition.set_signature(platform_value::BinaryData::new(vec![1, 2, 3]));
        assert_eq!(transition.signature().as_slice(), &[1, 2, 3]);

        transition.set_signature_bytes(vec![4, 5, 6]);
        assert_eq!(transition.signature().as_slice(), &[4, 5, 6]);
    }

    #[test]
    fn should_return_feature_version() {
        let data = get_test_data();
        assert_eq!(data.state_transition.feature_version(), 0);
    }

    #[test]
    fn should_set_data_contract_via_accessor() {
        let data = get_test_data();
        let original_contract = data.state_transition.data_contract().clone();

        let mut transition = data.state_transition;
        let modified_contract = original_contract.clone();
        transition.set_data_contract(modified_contract.clone());
        assert_eq!(transition.data_contract(), &modified_contract);
    }

    #[test]
    fn should_convert_to_object_with_and_without_signature() {
        let data = get_test_data();

        // to_object with signature
        let obj = StateTransitionValueConvert::to_object(&data.state_transition, false)
            .expect("to_object should succeed");
        assert!(obj.is_map());

        // to_object without signature
        let obj_no_sig = StateTransitionValueConvert::to_object(&data.state_transition, true)
            .expect("to_object skip_signature should succeed");
        assert!(obj_no_sig.is_map());
    }

    #[test]
    fn should_convert_to_cleaned_object() {
        let data = get_test_data();

        let cleaned = StateTransitionValueConvert::to_cleaned_object(&data.state_transition, false)
            .expect("to_cleaned_object should succeed");
        assert!(cleaned.is_map());

        let cleaned_no_sig =
            StateTransitionValueConvert::to_cleaned_object(&data.state_transition, true)
                .expect("to_cleaned_object skip_signature should succeed");
        assert!(cleaned_no_sig.is_map());
    }

    #[test]
    fn should_convert_to_canonical_object() {
        let data = get_test_data();

        let canonical =
            StateTransitionValueConvert::to_canonical_object(&data.state_transition, false)
                .expect("to_canonical_object should succeed");
        assert!(canonical.is_map());
    }

    #[test]
    fn should_convert_to_canonical_cleaned_object() {
        let data = get_test_data();

        let canonical_cleaned =
            StateTransitionValueConvert::to_canonical_cleaned_object(&data.state_transition, false)
                .expect("to_canonical_cleaned_object should succeed");
        assert!(canonical_cleaned.is_map());
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
    fn should_clean_value() {
        let data = get_test_data();

        let mut obj = StateTransitionValueConvert::to_object(&data.state_transition, false)
            .expect("to_object should succeed");

        obj.insert(STATE_TRANSITION_PROTOCOL_VERSION.to_string(), Value::U16(0))
            .expect("insert should succeed");

        let result =
            <DataContractCreateTransition as StateTransitionValueConvert>::clean_value(&mut obj);
        assert!(result.is_ok());
    }

    #[test]
    fn should_calculate_min_required_fee() {
        use crate::state_transition::StateTransitionEstimatedFeeValidation;

        let data = get_test_data();
        let fee = data
            .state_transition
            .calculate_min_required_fee(LATEST_PLATFORM_VERSION)
            .expect("fee calculation should succeed");

        // Fee should be positive (base_fee + registration_cost)
        assert!(fee > 0);
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
    fn should_convert_v0_to_state_transition() {
        let data = get_test_data();

        match &data.state_transition {
            DataContractCreateTransition::V0(v0) => {
                let st: StateTransition = v0.into();
                match st {
                    StateTransition::DataContractCreate(_) => {}
                    _ => panic!("expected DataContractCreate state transition"),
                }

                // Also test owned conversion
                let st_owned: StateTransition = v0.clone().into();
                match st_owned {
                    StateTransition::DataContractCreate(_) => {}
                    _ => panic!("expected DataContractCreate state transition"),
                }
            }
        }
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
    fn should_create_from_data_contract() {
        let created_data_contract = get_data_contract_fixture(None, 0, 1);
        let data_contract = created_data_contract.data_contract_owned();

        let transition = DataContractCreateTransition::try_from_platform_versioned(
            data_contract,
            LATEST_PLATFORM_VERSION,
        )
        .expect("should create transition from data contract");

        assert_eq!(transition.state_transition_version(), 0);
    }

    #[test]
    fn should_return_field_types() {
        let sig_paths = DataContractCreateTransition::signature_property_paths();
        assert_eq!(sig_paths.len(), 2);
        assert!(sig_paths.contains(&"signature"));
        assert!(sig_paths.contains(&"signaturePublicKeyId"));

        let id_paths = DataContractCreateTransition::identifiers_property_paths();
        assert!(id_paths.is_empty());

        let bin_paths = DataContractCreateTransition::binary_property_paths();
        assert_eq!(bin_paths.len(), 2);
    }

    #[test]
    fn v0_should_return_field_types() {
        let sig_paths = DataContractCreateTransitionV0::signature_property_paths();
        assert_eq!(sig_paths.len(), 2);

        let id_paths = DataContractCreateTransitionV0::identifiers_property_paths();
        assert!(id_paths.is_empty());

        let bin_paths = DataContractCreateTransitionV0::binary_property_paths();
        assert_eq!(bin_paths.len(), 1);
        assert!(bin_paths.contains(&"signature"));
    }

    #[test]
    fn v0_should_return_feature_version() {
        use crate::state_transition::FeatureVersioned;

        let data = get_test_data();
        match &data.state_transition {
            DataContractCreateTransition::V0(v0) => {
                assert_eq!(v0.feature_version(), 0);
            }
        }
    }

    #[test]
    fn v0_should_handle_identity_signed_methods() {
        use crate::identity::Purpose;
        use crate::identity::SecurityLevel;
        use crate::state_transition::StateTransitionIdentitySigned;

        let data = get_test_data();
        match data.state_transition {
            DataContractCreateTransition::V0(mut v0) => {
                assert_eq!(v0.signature_public_key_id(), 0);

                v0.set_signature_public_key_id(99);
                assert_eq!(v0.signature_public_key_id(), 99);

                let levels = v0.security_level_requirement(Purpose::AUTHENTICATION);
                assert!(levels.contains(&SecurityLevel::CRITICAL));
                assert!(levels.contains(&SecurityLevel::HIGH));
            }
        }
    }

    #[test]
    fn v0_should_handle_state_transition_like_methods() {
        use crate::state_transition::StateTransitionHasUserFeeIncrease;
        use crate::state_transition::StateTransitionSingleSigned;

        let data = get_test_data();
        match data.state_transition {
            DataContractCreateTransition::V0(mut v0) => {
                // modified_data_ids
                let ids = v0.modified_data_ids();
                assert_eq!(ids.len(), 1);

                // state_transition_protocol_version
                assert_eq!(v0.state_transition_protocol_version(), 0);

                // state_transition_type
                assert_eq!(
                    v0.state_transition_type(),
                    StateTransitionType::DataContractCreate
                );

                // unique_identifiers
                let uids = v0.unique_identifiers();
                assert_eq!(uids.len(), 1);
                assert!(uids[0].starts_with("dcc-"));

                // owner_id
                let _owner = v0.owner_id();

                // user_fee_increase
                assert_eq!(v0.user_fee_increase(), 0);
                v0.set_user_fee_increase(10);
                assert_eq!(v0.user_fee_increase(), 10);

                // signature
                assert!(v0.signature().is_empty());
                v0.set_signature(platform_value::BinaryData::new(vec![7, 8, 9]));
                assert_eq!(v0.signature().as_slice(), &[7, 8, 9]);

                v0.set_signature_bytes(vec![10, 11, 12]);
                assert_eq!(v0.signature().as_slice(), &[10, 11, 12]);
            }
        }
    }

    #[test]
    fn v0_should_convert_to_object_with_skip_signature() {
        let data = get_test_data();
        match &data.state_transition {
            DataContractCreateTransition::V0(v0) => {
                let obj = v0.to_object(false).expect("to_object should succeed");
                assert!(obj.is_map());

                let obj_no_sig = v0
                    .to_object(true)
                    .expect("to_object skip_signature should succeed");
                assert!(obj_no_sig.is_map());
            }
        }
    }

    #[test]
    fn v0_should_convert_to_cleaned_object() {
        let data = get_test_data();
        match &data.state_transition {
            DataContractCreateTransition::V0(v0) => {
                let cleaned = v0
                    .to_cleaned_object(false)
                    .expect("to_cleaned_object should succeed");
                assert!(cleaned.is_map());

                let cleaned_no_sig = v0
                    .to_cleaned_object(true)
                    .expect("to_cleaned_object skip_signature should succeed");
                assert!(cleaned_no_sig.is_map());
            }
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
    fn v0_should_clean_value() {
        let data = get_test_data();
        match &data.state_transition {
            DataContractCreateTransition::V0(v0) => {
                let mut obj = v0.to_object(false).expect("to_object should succeed");
                DataContractCreateTransitionV0::clean_value(&mut obj)
                    .expect("clean_value should succeed");
            }
        }
    }

    #[test]
    fn v0_should_create_from_data_contract() {
        let created_data_contract = get_data_contract_fixture(None, 0, 1);
        let data_contract = created_data_contract.data_contract_owned();

        let v0 = DataContractCreateTransitionV0::try_from_platform_versioned(
            data_contract,
            LATEST_PLATFORM_VERSION,
        )
        .expect("should create v0 from data contract");

        assert_eq!(v0.user_fee_increase, 0);
        assert_eq!(v0.signature_public_key_id, 0);
        assert!(v0.signature.is_empty());
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
