use super::DataContractInSerializationFormatV2;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::serialized_version::v1::DataContractInSerializationFormatV1;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::DataContract;
use crate::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructureTrusted,
    PlatformSerializableWithPlatformVersion,
};
use crate::tests::fixtures::get_data_contract_fixture;
use platform_serialization::PLATFORM_BINCODE_CONFIG;
use platform_version::version::{PlatformVersion, PLATFORM_VERSIONS};
use platform_version::TryIntoPlatformVersioned;

fn fixture_contract() -> DataContract {
    get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
        .data_contract_owned()
}

fn stored(contract: &DataContract) -> DataContractInSerializationFormatV2 {
    match DataContractInSerializationFormat::try_from_data_contract_for_storage(
        contract,
        PlatformVersion::latest(),
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

/// The stored form reads back as the contract it was compiled from at every protocol version
/// whose contracts have the structure it reads into: its document types are read, not parsed,
/// so the version reading them changes nothing.
#[test]
fn should_read_the_stored_form_back_at_every_protocol_version_of_its_structure() {
    let contract = fixture_contract();
    let bytes = contract
        .serialize_to_storage_bytes(PlatformVersion::latest())
        .expect("expected the contract to compile");
    let structure = PlatformVersion::latest()
        .dpp
        .contract_versions
        .contract_structure_version;
    for platform_version in PLATFORM_VERSIONS
        .iter()
        .filter(|version| version.dpp.contract_versions.contract_structure_version == structure)
    {
        let protocol_version = platform_version.protocol_version;
        let read_back =
            DataContract::versioned_deserialize_trusted(&bytes, false, platform_version)
                .unwrap_or_else(|error| panic!("protocol version {protocol_version}: {error}"));
        assert_eq!(read_back, contract, "protocol version {protocol_version}");
    }
}

/// Each protocol version stores a contract in the format its table selects
/// (`contract_storage_serialization_version`); in format version 0 or 1 the stored bytes are
/// the bytes every other serialization gives at that version.
#[test]
fn should_store_a_contract_in_the_format_each_protocol_version_selects() {
    let contract = fixture_contract();
    for platform_version in PLATFORM_VERSIONS {
        // A version whose serialization refuses the fixture's configuration stores nothing
        let Ok(serialized) = contract.serialize_to_bytes_with_platform_version(platform_version)
        else {
            continue;
        };
        let stored = contract
            .serialize_to_storage_bytes(platform_version)
            .expect("expected the contract to store");
        let selected = platform_version
            .dpp
            .contract_versions
            .contract_storage_serialization_version;
        let protocol_version = platform_version.protocol_version;
        // The format's variant index, a single byte below 251
        assert_eq!(
            u16::from(stored[0]),
            selected,
            "protocol version {protocol_version}"
        );
        if selected < 2 {
            assert_eq!(stored, serialized, "protocol version {protocol_version}");
        }
    }
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

/// `contract` stored, with the compiled forms of two of its types swapped: compiled types that
/// are not those of its schemas, which only bytes from someone other than a node hold.
fn stored_with_swapped_types(contract: &DataContract) -> DataContractInSerializationFormatV2 {
    let mut format = stored(contract);
    let names: Vec<String> = format.document_types.keys().cloned().collect();
    assert!(names.len() >= 2, "expected two document types");
    let first = format.document_types[&names[0]].clone();
    let second = format.document_types[&names[1]].clone();
    format.document_types.insert(names[0].clone(), second);
    format.document_types.insert(names[1].clone(), first);
    format
}

/// Fully validated, a stored contract is parsed from its schemas, which are checked, and its
/// compiled types must be what they compile to: the contract read is the one state holds, or
/// none.
#[test]
fn should_refuse_a_fully_validated_stored_form_whose_compiled_types_its_schemas_do_not_compile_to()
{
    let contract = fixture_contract();
    let latest = PlatformVersion::latest();
    let read = DataContract::try_from_platform_versioned(
        DataContractInSerializationFormat::V2(stored(&contract)),
        true,
        &mut vec![],
        latest,
    )
    .expect("expected the stored form to read");
    assert_eq!(read, contract);

    let error = DataContract::try_from_platform_versioned(
        DataContractInSerializationFormat::V2(stored_with_swapped_types(&contract)),
        true,
        &mut vec![],
        latest,
    )
    .expect_err("expected the stored form to be refused");
    assert!(
        error.to_string().contains("is not what protocol version"),
        "{error}"
    );
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
    let format = DataContractInSerializationFormat::V2(stored_with_swapped_types(&contract));
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
    // Fully validated, the untrusted decoders check the compiled types instead
    assert!(DataContract::versioned_deserialize_untrusted(&bytes, true, latest).is_err());
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

/// A stored contract's schemas are not changed in place, which would leave compiled types that
/// are not theirs; its format version 1's are.
#[test]
fn should_refuse_to_change_the_schemas_of_a_stored_form_in_place() {
    let contract = fixture_contract();
    let mut format = DataContractInSerializationFormat::V2(stored(&contract));
    assert!(format.document_schemas_mut().is_err());
    let mut format = format.into_schema_form();
    let schemas = format
        .document_schemas_mut()
        .expect("expected format version 1's schemas");
    let name = schemas
        .keys()
        .next()
        .expect("expected a document type")
        .clone();
    schemas.remove(&name);
    assert!(!format.document_schemas().contains_key(&name));
}

/// The JSON and platform value entry points read a stored contract as the untrusted decoders
/// do: from its schemas, and fully validated, with its compiled types checked against them.
#[cfg(feature = "json-conversion")]
#[test]
fn should_read_a_stored_form_from_json_as_the_untrusted_decoders_do() {
    use crate::data_contract::conversion::json::DataContractJsonConversionMethodsV0;

    let contract = fixture_contract();
    let latest = PlatformVersion::latest();
    let json = serde_json::to_value(DataContractInSerializationFormat::V2(stored(&contract)))
        .expect("expected the stored form to convert to JSON");
    for full_validation in [false, true] {
        let read = DataContract::from_json(json.clone(), full_validation, latest)
            .expect("expected the stored form to read from JSON");
        assert_eq!(
            read.id(),
            contract.id(),
            "full validation {full_validation}"
        );
        assert_eq!(
            read.document_types().keys().collect::<Vec<_>>(),
            contract.document_types().keys().collect::<Vec<_>>(),
            "full validation {full_validation}"
        );
    }
    let swapped = serde_json::to_value(DataContractInSerializationFormat::V2(
        stored_with_swapped_types(&contract),
    ))
    .expect("expected the stored form to convert to JSON");
    assert!(DataContract::from_json(swapped.clone(), false, latest).is_ok());
    assert!(DataContract::from_json(swapped, true, latest).is_err());
}
