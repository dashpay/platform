pub mod accessors;
mod fields;
mod identity_signed;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
mod v0;
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
    use crate::data_contract::created_data_contract::CreatedDataContract;

    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
    use crate::state_transition::traits::StateTransitionLike;
    use crate::state_transition::{StateTransitionOwned, StateTransitionType};
    use crate::tests::fixtures::get_data_contract_fixture;

    use crate::version::LATEST_PLATFORM_VERSION;

    pub(crate) struct TestData {
        pub(crate) state_transition: DataContractCreateTransition,
        pub(crate) created_data_contract: CreatedDataContract,
    }

    pub(crate) fn get_test_data() -> TestData {
        let created_data_contract = get_data_contract_fixture(None, 0, 1);

        let state_transition = DataContractCreateTransition::try_from_platform_versioned(
            created_data_contract.clone(),
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
            serde_json::to_value(&data_contract).expect("conversion to object shouldn't fail"),
            serde_json::to_value(data.created_data_contract.data_contract())
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

    // Legacy `StateTransitionValueConvert` round-trip tests deleted in
    // Phase D step 9. The canonical `JsonConvertible` / `ValueConvertible`
    // round-trip is exercised on the outer enum derive — the legacy
    // round-trip via `to_object(false)` + `from_object(value, pv)` was
    // testing methods that no longer exist.

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

    // V0 legacy round-trip tests deleted in Phase D step 9 — they were
    // exercising deleted `StateTransitionValueConvert` methods. Outer-enum
    // canonical round-trip in `json_convertible_tests` covers correctness.

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

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
pub(crate) mod json_convertible_tests {
    use super::*;
    use crate::state_transition::data_contract_create_transition::v0::DataContractCreateTransitionV0;
    use crate::tests::fixtures::get_data_contract_fixture;
    use platform_value::BinaryData;
    use platform_version::version::PlatformVersion;
    use platform_version::TryFromPlatformVersioned;

    pub(crate) fn fixture() -> DataContractCreateTransition {
        let pv = PlatformVersion::latest();
        let created = get_data_contract_fixture(None, 0, pv.protocol_version);
        let data_contract = created.data_contract().clone();
        let mut v0 = DataContractCreateTransitionV0::try_from_platform_versioned(data_contract, pv)
            .expect("v0 from contract");
        v0.identity_nonce = 5;
        v0.user_fee_increase = 3;
        v0.signature_public_key_id = 1;
        v0.signature = BinaryData::new(vec![0xab; 65]);
        DataContractCreateTransition::V0(v0)
    }

    fn assert_v0_fields(t: &DataContractCreateTransition) {
        let DataContractCreateTransition::V0(rec) = t;
        assert_eq!(rec.identity_nonce, 5, "identity_nonce");
        assert_eq!(rec.user_fee_increase, 3, "user_fee_increase");
        assert_eq!(rec.signature_public_key_id, 1, "signature_public_key_id");
        assert_eq!(rec.signature, BinaryData::new(vec![0xab; 65]), "signature");
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        // JSON has a single `Number` type, so sized integer variants in the
        // `document_schemas` Value tree (e.g. `U32(63)`, `I32(0)`) collapse to
        // `U64` on round-trip — a fundamental serde_json limitation, not a bug.
        // We compare under a normalization that projects both sides through the
        // same lossy map. See `tests::utils::normalize_integer_variants_for_json_round_trip`.
        use crate::serialization::{JsonConvertible, ValueConvertible};
        use crate::tests::utils::normalize_integer_variants_for_json_round_trip;
        let original = fixture();
        let json = JsonConvertible::to_json(&original).expect("to_json");
        let recovered =
            <DataContractCreateTransition as JsonConvertible>::from_json(json).expect("from_json");
        let mut original_canon = ValueConvertible::to_object(&original).expect("to_object");
        let mut recovered_canon = ValueConvertible::to_object(&recovered).expect("to_object");
        normalize_integer_variants_for_json_round_trip(&mut original_canon);
        normalize_integer_variants_for_json_round_trip(&mut recovered_canon);
        assert_eq!(original_canon, recovered_canon);
        assert_v0_fields(&recovered);
    }

    #[test]
    fn json_preserves_format_version_tag() {
        use crate::serialization::JsonConvertible;
        let json = JsonConvertible::to_json(&fixture()).expect("to_json");
        assert_eq!(json["$formatVersion"], "0");
    }

    #[test]
    fn value_round_trip_with_envelope_wire_shape() {
        use crate::serialization::ValueConvertible;
        use platform_value::{platform_value, Value};
        let original = fixture();
        let value = ValueConvertible::to_object(&original).expect("to_object");
        // Tier 3 envelope-only: the inner `dataContract` is a fully-fledged
        // versioned `DataContractInSerializationFormat` with embedded JSON
        // Schemas, group / token / keyword maps, etc. — far too large to inline
        // here. We assert the outer envelope shape (every non-`dataContract`
        // field) with sized-int suffixes (`5u64` for `identityNonce` u64,
        // `3u16` for `userFeeIncrease` u16, `1u32` for `signaturePublicKeyId`
        // u32, `BinaryData` -> `Value::Bytes`), and check that the
        // `dataContract` slot is a `Value::Map` (its full shape is exercised
        // by the round-trip-equality assertion further down + the tests living
        // alongside `DataContractInSerializationFormat`).
        let envelope: std::collections::BTreeMap<String, Value> = match &value {
            Value::Map(entries) => entries
                .iter()
                .filter_map(|(k, v)| match k {
                    Value::Text(s) if s != "dataContract" => Some((s.clone(), v.clone())),
                    _ => None,
                })
                .collect(),
            _ => panic!("value is not a Map"),
        };
        let envelope_value: Value = envelope.into();
        // Note: assertion uses alphabetical key order — BTreeMap sorts.
        assert_eq!(
            envelope_value,
            platform_value!({
                "$formatVersion": "0",
                "identityNonce": 5u64,
                "signature": Value::Bytes(vec![0xab; 65]),
                "signaturePublicKeyId": 1u32,
                "userFeeIncrease": 3u16,
            })
        );
        // dataContract slot is present and is a Map (full inline shape skipped)
        let has_data_contract = matches!(
            &value,
            Value::Map(entries) if entries.iter().any(|(k, v)|
                matches!(k, Value::Text(s) if s == "dataContract") &&
                matches!(v, Value::Map(_))
            )
        );
        assert!(has_data_contract, "dataContract slot must be a Value::Map");
        let recovered = <DataContractCreateTransition as ValueConvertible>::from_object(value)
            .expect("from_object");
        assert_eq!(original, recovered);
    }
}
