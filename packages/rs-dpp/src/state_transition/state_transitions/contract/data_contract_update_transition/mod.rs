#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::state_transition::StateTransitionFieldTypes;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize, PlatformSignable,
};

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
mod v1;
mod version;

pub use fields::*;
use platform_version::version::PlatformVersion;
use platform_version::{TryFromPlatformVersioned, TryIntoPlatformVersioned};

use crate::data_contract::DataContract;

use crate::identity::state_transition::OptionallyAssetLockProved;
use crate::prelude::IdentityNonce;
pub use v0::*;
pub use v1::*;

pub type DataContractUpdateTransitionLatest = DataContractUpdateTransitionV1;

#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    PlatformSerialize,
    PlatformSignable,
    PlatformVersioned,
    From,
    PartialEq,
    DecodeUntrusted,
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
    #[cfg_attr(feature = "serde-conversion", serde(rename = "1"))]
    V1(DataContractUpdateTransitionV1),
}

/// A lone contract can only become a full-contract (V0) update: the delta
/// form needs the stored contract too, see
/// [`DataContractUpdateTransition::from_contract_update`].
impl TryFromPlatformVersioned<(DataContract, IdentityNonce)> for DataContractUpdateTransition {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: (DataContract, IdentityNonce),
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        let data_contract_update_transition: DataContractUpdateTransitionV0 =
            value.try_into_platform_versioned(platform_version)?;
        Ok(data_contract_update_transition.into())
    }
}

impl DataContractUpdateTransition {
    /// Builds the unsigned update that turns `old_contract` into
    /// `new_contract`, in the form the platform version defaults to: a
    /// full-contract V0 transition, or a delta-based V1 one.
    pub fn from_contract_update(
        old_contract: &DataContract,
        new_contract: &DataContract,
        identity_contract_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .contract_update_state_transition
            .default_current_version
        {
            0 => {
                let data_contract_update_transition: DataContractUpdateTransitionV0 =
                    (new_contract.clone(), identity_contract_nonce)
                        .try_into_platform_versioned(platform_version)?;
                Ok(data_contract_update_transition.into())
            }
            1 => Ok(DataContractUpdateTransitionV1::from_contract_update(
                old_contract,
                new_contract,
                identity_contract_nonce,
            )?
            .into()),
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
    use crate::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
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
            PlatformVersion::first()
                .dpp
                .state_transition_serialization_versions
                .contract_update_state_transition
                .default_current_version,
            data.state_transition.state_transition_protocol_version()
        )
    }

    #[test]
    fn should_build_a_delta_by_default_at_the_latest_platform_version() {
        let platform_version = PlatformVersion::latest();
        let old_contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let mut new_contract = old_contract.clone();
        new_contract.increment_version();

        let state_transition = DataContractUpdateTransition::from_contract_update(
            &old_contract,
            &new_contract,
            1,
            platform_version,
        )
        .expect("expected a delta-based update transition");

        assert_eq!(
            LATEST_PLATFORM_VERSION
                .dpp
                .state_transition_serialization_versions
                .contract_update_state_transition
                .default_current_version,
            state_transition.state_transition_protocol_version()
        );
        assert!(matches!(
            state_transition,
            DataContractUpdateTransition::V1(_)
        ));
        assert_eq!(state_transition.data_contract_id(), old_contract.id());
        assert_eq!(state_transition.owner_id(), old_contract.owner_id());
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
            data.state_transition
                .data_contract()
                .expect("a V0 update embeds its contract")
                .clone(),
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
        let DataContractUpdateTransition::V0(rec) = t else {
            panic!("expected a V0 update transition");
        };
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

    /// A delta holding each kind of field its two serialized forms write differently: the
    /// nonce, identifiers, a group keyed by its position and members keyed by identity.
    fn delta_fixture() -> DataContractUpdateTransition {
        use crate::data_contract::group::v0::GroupV0;
        use crate::data_contract::group::Group;
        use crate::data_contract::update_values::DescriptionUpdate;
        use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV1;
        use platform_value::Identifier;
        use std::collections::BTreeMap;

        DataContractUpdateTransition::V1(DataContractUpdateTransitionV1 {
            identity_contract_nonce: 8,
            data_contract_id: Identifier::new([1; 32]),
            owner_id: Identifier::new([2; 32]),
            version: 2,
            config: None,
            updated_schema_defs: BTreeMap::new(),
            new_schema_defs: BTreeMap::new(),
            updated_document_schemas: BTreeMap::new(),
            new_document_schemas: BTreeMap::new(),
            new_groups: BTreeMap::from([(
                0,
                Group::V0(GroupV0 {
                    members: BTreeMap::from([
                        (Identifier::new([3; 32]), 1),
                        (Identifier::new([4; 32]), 1),
                    ]),
                    required_power: 2,
                }),
            )]),
            new_tokens: BTreeMap::new(),
            add_keywords: vec!["wallet".to_string()],
            remove_keywords: vec![],
            description: DescriptionUpdate::Set("a described contract".to_string()),
            user_fee_increase: 5,
            signature_public_key_id: 1,
            signature: BinaryData::new(vec![0xff; 65]),
        })
    }

    /// The JSON form of a delta, which the WASM bindings hand to JavaScript as it is: the
    /// nonce a number while it is a safe integer, identifiers as base58 text, a group a
    /// plain record under its position.
    #[test]
    fn should_write_a_delta_as_json_with_text_identifiers_and_a_numeric_nonce() {
        use crate::serialization::JsonConvertible;
        use platform_value::string_encoding::Encoding;
        use platform_value::Identifier;
        use serde_json::json;

        let original = delta_fixture();
        let json = JsonConvertible::to_json(&original).expect("to_json");

        assert_eq!(json["$formatVersion"], "1");
        assert_eq!(json["$identity-contract-nonce"], json!(8));
        assert_eq!(
            json["dataContractId"],
            json!(Identifier::new([1; 32]).to_string(Encoding::Base58))
        );
        assert_eq!(
            json["ownerId"],
            json!(Identifier::new([2; 32]).to_string(Encoding::Base58))
        );
        assert_eq!(json["version"], json!(2));
        assert_eq!(json["newGroups"]["0"]["requiredPower"], json!(2));
        assert_eq!(
            json["newGroups"]["0"]["members"][Identifier::new([3; 32]).to_string(Encoding::Base58)],
            json!(1)
        );
        assert_eq!(json["addKeywords"], json!(["wallet"]));
        assert_eq!(json["description"], json!({"set": "a described contract"}));

        let recovered =
            <DataContractUpdateTransition as JsonConvertible>::from_json(json).expect("from_json");
        assert_eq!(recovered, original);
    }

    /// The object form of a delta, which the WASM bindings turn into a JavaScript object
    /// value by value: identifiers are identifier values (bytes there, not wrapper
    /// instances), the nonce an integer of 64 bits (a bigint there), a group a plain map.
    /// A group's position is an integer key here; the bindings write and read it as text,
    /// the form the group and token maps are read back in, as a contract's are.
    #[test]
    fn should_write_a_delta_as_an_object_with_byte_identifiers_and_a_plain_group() {
        use crate::serialization::ValueConvertible;
        use platform_value::{Identifier, Value};

        let original = delta_fixture();
        let object = ValueConvertible::to_object(&original).expect("to_object");
        fn field_of<'a>(object: &'a Value, name: &'a str) -> &'a Value {
            object
                .get(name)
                .expect("expected a map")
                .unwrap_or_else(|| panic!("expected the field {name}"))
        }
        let field = |name: &'static str| field_of(&object, name);

        assert_eq!(field("$formatVersion"), &Value::Text("1".to_string()));
        assert_eq!(field("$identity-contract-nonce"), &Value::U64(8));
        assert_eq!(
            field("dataContractId"),
            &Value::Identifier(Identifier::new([1; 32]).to_buffer())
        );
        assert_eq!(
            field("ownerId"),
            &Value::Identifier(Identifier::new([2; 32]).to_buffer())
        );
        let Value::Map(groups) = field("newGroups") else {
            panic!("expected the new groups as a map");
        };
        let [(position, Value::Map(group))] = groups.as_slice() else {
            panic!("expected one group, a plain map: {groups:?}");
        };
        assert_eq!(
            position.to_integer::<u16>().expect("expected a position"),
            0
        );
        assert!(group.iter().any(
            |(key, value)| key == &Value::Text("requiredPower".to_string())
                && value.to_integer::<u32>().ok() == Some(2)
        ));
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
