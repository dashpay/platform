use super::DataContractInSerializationFormatV2;
use crate::data_contract::serialized_version::v1::DataContractInSerializationFormatV1;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::DataContract;
use crate::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted,
    PlatformSerializableWithPlatformVersion,
};
use crate::tests::fixtures::get_data_contract_fixture;
use platform_serialization::PLATFORM_BINCODE_CONFIG;
use platform_version::version::PlatformVersion;
use platform_version::TryIntoPlatformVersioned;
use std::sync::OnceLock;

/// The latest protocol version, but storing contracts in format version 2.
fn storing_compiled_contracts() -> &'static PlatformVersion {
    static VERSION: OnceLock<PlatformVersion> = OnceLock::new();
    VERSION.get_or_init(|| {
        let mut version = PlatformVersion::latest().clone();
        version
            .dpp
            .contract_versions
            .contract_storage_serialization_version = 2;
        version
    })
}

fn fixture_contract() -> DataContract {
    get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
        .data_contract_owned()
}

fn stored(contract: &DataContract) -> DataContractInSerializationFormatV2 {
    match DataContractInSerializationFormat::try_from_data_contract_for_storage(
        contract.clone(),
        storing_compiled_contracts(),
    )
    .expect("expected the contract to compile")
    {
        DataContractInSerializationFormat::V2(v2) => v2,
        other => panic!("expected format version 2, got {other:?}"),
    }
}

#[test]
fn should_give_back_the_contract_format_version_1_from_its_stored_form() {
    let contract = fixture_contract();
    let platform_version = PlatformVersion::latest();
    let format_v1: DataContractInSerializationFormat = contract
        .clone()
        .try_into_platform_versioned(platform_version)
        .expect("expected the contract to serialize");
    let DataContractInSerializationFormat::V1(format_v1) = format_v1 else {
        panic!("expected format version 1");
    };
    assert_eq!(
        DataContractInSerializationFormatV1::from(stored(&contract)),
        format_v1
    );
}

#[test]
fn should_read_the_stored_form_back_at_protocol_versions_13_to_15() {
    let contract = fixture_contract();
    let bytes = contract
        .serialize_to_storage_bytes(storing_compiled_contracts())
        .expect("expected the contract to compile");
    for protocol_version in 13..=15 {
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected the protocol version");
        let read_back =
            DataContract::versioned_deserialize_trusted(&bytes, false, platform_version)
                .unwrap_or_else(|error| panic!("protocol version {protocol_version}: {error}"));
        assert_eq!(read_back, contract, "protocol version {protocol_version}");
    }
}

#[test]
fn should_store_format_version_1_until_a_protocol_version_selects_the_stored_form() {
    let contract = fixture_contract();
    let platform_version = PlatformVersion::latest();
    assert_eq!(
        contract
            .serialize_to_storage_bytes(platform_version)
            .expect("expected the contract to serialize"),
        contract
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("expected the contract to serialize"),
    );
}

#[test]
fn should_refuse_a_stored_form_whose_compiled_types_are_not_those_of_its_schemas() {
    let contract = fixture_contract();
    let mut format = stored(&contract);
    let name = format
        .document_types
        .keys()
        .next()
        .expect("expected a document type")
        .clone();
    format.document_types.remove(&name);
    // As Drive reads it
    let error = DataContract::try_from_platform_versioned(
        DataContractInSerializationFormat::V2(format.clone()),
        false,
        &mut vec![],
        PlatformVersion::latest(),
    )
    .expect_err("expected the stored form to be refused");
    assert!(
        error.to_string().contains("not those of its schemas"),
        "{error}"
    );
}

/// Fully validated, a stored contract is read as its format version 1: its compiled types are
/// not read, so ones that do not match its schemas, from bytes anyone could have written, change
/// nothing.
#[test]
fn should_read_a_fully_validated_stored_form_from_its_schemas_alone() {
    let contract = fixture_contract();
    let mut format = stored(&contract);
    let names: Vec<String> = format.document_types.keys().cloned().collect();
    assert!(names.len() >= 2, "expected two document types");
    // Swap two types' compiled forms
    let first = format.document_types[&names[0]].clone();
    let second = format.document_types[&names[1]].clone();
    format.document_types.insert(names[0].clone(), second);
    format.document_types.insert(names[1].clone(), first);
    let read = DataContract::try_from_platform_versioned(
        DataContractInSerializationFormat::V2(format),
        true,
        &mut vec![],
        PlatformVersion::latest(),
    )
    .expect("expected the stored form to read from its schemas");
    assert_eq!(read, contract);
}

/// The untrusted decoders read bytes nothing vouches for from their schemas, so compiled types
/// that do not match them change nothing there; read as Drive reads its state, or as proof
/// verification reads authenticated state, the same bytes give the compiled types back as they
/// are.
#[test]
fn should_read_an_untrusted_stored_form_from_its_schemas_alone() {
    use crate::data_contract::created_data_contract::v0::CreatedDataContractInSerializationFormatV0;
    use crate::data_contract::created_data_contract::{
        CreatedDataContract, CreatedDataContractInSerializationFormat,
    };
    use crate::serialization::{
        PlatformDeserializableWithBytesLenFromVersionedStructureUntrusted,
        PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted,
    };

    let contract = fixture_contract();
    let mut format = stored(&contract);
    let names: Vec<String> = format.document_types.keys().cloned().collect();
    assert!(names.len() >= 2, "expected two document types");
    // Swap two types' compiled forms
    let first = format.document_types[&names[0]].clone();
    let second = format.document_types[&names[1]].clone();
    format.document_types.insert(names[0].clone(), second);
    format.document_types.insert(names[1].clone(), first);
    let format = DataContractInSerializationFormat::V2(format);
    let config = PLATFORM_BINCODE_CONFIG;
    let bytes =
        bincode::encode_to_vec(&format, config).expect("expected the stored form to encode");
    let latest = PlatformVersion::latest();

    let untrusted = DataContract::versioned_deserialize_untrusted(&bytes, false, latest)
        .expect("expected the stored form to read from its schemas");
    assert_eq!(untrusted, contract);
    let (untrusted_with_len, _) =
        DataContract::versioned_deserialize_with_bytes_len_untrusted(&bytes, false, latest)
            .expect("expected the stored form to read from its schemas");
    assert_eq!(untrusted_with_len, contract);
    // A contract about to be registered is read from its schemas, whatever reads it
    let created_bytes = bincode::encode_to_vec(
        CreatedDataContractInSerializationFormat::V0(CreatedDataContractInSerializationFormatV0 {
            data_contract: format.clone(),
            identity_nonce: 1,
        }),
        config,
    )
    .expect("expected the created contract to encode");
    for created in [
        CreatedDataContract::versioned_deserialize_trusted(&created_bytes, false, latest),
        CreatedDataContract::versioned_deserialize_untrusted(&created_bytes, false, latest),
    ] {
        assert_eq!(
            created
                .expect("expected the created contract to read from its schemas")
                .data_contract(),
            &contract
        );
    }

    for read_back in [
        DataContract::versioned_deserialize_trusted(&bytes, false, latest),
        DataContract::try_from_platform_versioned(format, false, &mut vec![], latest),
    ] {
        assert_ne!(
            read_back.expect("expected the stored form to read back"),
            contract
        );
    }
}

/// JSON carries the compiled document types as bytes, so they come back exactly; the raw
/// schemas come back as JSON gives any format's schemas back, with their integers widened.
/// A platform value gives everything back.
#[cfg(feature = "serde-conversion")]
#[test]
fn should_round_trip_the_stored_form_through_json_and_platform_value() {
    let compiled = stored(&fixture_contract());
    let format = DataContractInSerializationFormat::V2(compiled.clone());
    let json = serde_json::to_value(&format).expect("expected the format to convert to JSON");
    assert_eq!(json["$formatVersion"], "2");
    let from_json: DataContractInSerializationFormat =
        serde_json::from_value(json).expect("expected the format to convert from JSON");
    let DataContractInSerializationFormat::V2(from_json) = from_json else {
        panic!("expected format version 2");
    };
    assert_eq!(from_json.document_types, compiled.document_types);

    let value = platform_value::to_value(&format).expect("expected the format to convert");
    let from_value: DataContractInSerializationFormat =
        platform_value::from_value(value).expect("expected the format to convert back");
    assert_eq!(from_value, format);
}

/// Changing a stored contract's schemas drops its compiled types: it becomes its format version
/// 1, so it never holds compiled types that are not those of its schemas.
#[test]
fn should_drop_the_compiled_types_of_a_stored_form_whose_schemas_change() {
    let contract = fixture_contract();
    let compiled = stored(&contract);
    let mut format = DataContractInSerializationFormat::V2(compiled.clone());
    let name = compiled
        .document_schemas
        .keys()
        .next()
        .expect("expected a document type")
        .clone();
    format.document_schemas_mut().remove(&name);
    let DataContractInSerializationFormat::V1(changed) = format else {
        panic!("expected format version 1, got {format:?}");
    };
    let mut expected = DataContractInSerializationFormatV1::from(compiled);
    expected.document_schemas.remove(&name);
    assert_eq!(changed, expected);
}
