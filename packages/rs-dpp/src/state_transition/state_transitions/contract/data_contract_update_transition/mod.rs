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
pub mod methods;
mod serialize;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
mod v0;
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

    // Legacy `StateTransitionValueConvert` / `StateTransitionJsonConvert`
    // round-trip tests deleted in Phase D step 9. The canonical
    // `JsonConvertible` / `ValueConvertible` round-trip is exercised on the
    // outer enum derive (see `json_convertible_tests` below) — these tested
    // methods that no longer exist.
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
pub(crate) mod json_convertible_tests {
    use super::*;
    use crate::state_transition::data_contract_update_transition::v0::DataContractUpdateTransitionV0;
    use crate::tests::fixtures::get_data_contract_fixture;
    use platform_value::BinaryData;
    use platform_version::version::PlatformVersion;
    use platform_version::TryIntoPlatformVersioned;

    pub(crate) fn fixture() -> DataContractUpdateTransition {
        let pv = PlatformVersion::latest();
        let created = get_data_contract_fixture(None, 0, pv.protocol_version);
        let data_contract = created.data_contract().clone();
        let data_contract_format = data_contract
            .try_into_platform_versioned(pv)
            .expect("contract -> format");
        DataContractUpdateTransition::V0(DataContractUpdateTransitionV0 {
            identity_contract_nonce: 8,
            data_contract: data_contract_format,
            user_fee_increase: 5,
            signature_public_key_id: 1,
            signature: BinaryData::new(vec![0xff; 65]),
        })
    }

    fn assert_v0_fields(t: &DataContractUpdateTransition) {
        let DataContractUpdateTransition::V0(rec) = t;
        assert_eq!(rec.identity_contract_nonce, 8, "identity_contract_nonce");
        assert_eq!(rec.user_fee_increase, 5, "user_fee_increase");
        assert_eq!(rec.signature_public_key_id, 1, "signature_public_key_id");
        assert_eq!(rec.signature, BinaryData::new(vec![0xff; 65]), "signature");
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        // JSON's single `Number` type erases sized-int variants in the
        // `document_schemas` tree on round-trip. Compare under normalization;
        // see `tests::utils::normalize_integer_variants_for_json_round_trip`.
        use crate::serialization::{JsonConvertible, ValueConvertible};
        use crate::tests::utils::normalize_integer_variants_for_json_round_trip;
        let original = fixture();
        let json = JsonConvertible::to_json(&original).expect("to_json");
        let recovered =
            <DataContractUpdateTransition as JsonConvertible>::from_json(json).expect("from_json");
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
        // field) with sized-int suffixes (`8u64` for `$identity-contract-nonce`
        // u64 — yes, kebab-case-with-leading-dollar is the actual wire key,
        // see `crate::state_transition::state_transitions::contract::property_names`,
        // `5u16` for `userFeeIncrease` u16, `1u32` for `signaturePublicKeyId`
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
                "$identity-contract-nonce": 8u64,
                "signature": Value::Bytes(vec![0xff; 65]),
                "signaturePublicKeyId": 1u32,
                "userFeeIncrease": 5u16,
            })
        );
        let has_data_contract = matches!(
            &value,
            Value::Map(entries) if entries.iter().any(|(k, v)|
                matches!(k, Value::Text(s) if s == "dataContract") &&
                matches!(v, Value::Map(_))
            )
        );
        assert!(has_data_contract, "dataContract slot must be a Value::Map");
        let recovered = <DataContractUpdateTransition as ValueConvertible>::from_object(value)
            .expect("from_object");
        assert_eq!(original, recovered);
    }
}
