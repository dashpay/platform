use crate::drive::votes::storage_form::contested_document_resource_reference_storage_form::ContestedDocumentResourceVoteReferenceStorageForm;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::bincode;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::platform_serialization::PLATFORM_BINCODE_CONFIG;
use dpp::prelude::DataContract;
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;

/// Maximum decoded resource budget for a proof-derived vote reference.
///
/// Canonical vote references contain only a short, bounded Drive path. This
/// leaves substantial compatibility headroom while preventing compact length
/// prefixes from requesting attacker-selected allocations.
const MAX_VOTE_REFERENCE_DECODE_BYTES: usize = 64 * 1024;

/// Maximum in-memory decode budget for a proof-derived data contract.
///
/// Contract input bytes are separately capped by the active protocol version: in the format
/// state transitions carry by `max_serialized_size` (currently 65,000 bytes), and stored
/// compiled (format version 2, from a protocol version whose storage selector is 2) by
/// `max_compiled_contract_size` (1 MiB). Bincode charges this budget for decoded containers
/// rather than wire bytes, so the larger value preserves ample compatibility headroom while
/// still bounding hostile compact length prefixes before a proof root is trusted.
const MAX_CONTRACT_DECODE_MEMORY_BYTES: usize = 16 * 1024 * 1024;

/// The first byte of a contract stored in format version 2, the compiled form: its format
/// version, which bincode writes as a single byte.
const COMPILED_CONTRACT_FORMAT_TAG: u8 = 2;

pub(super) fn decode_proof_data_contract(
    serialized_contract: &[u8],
    platform_version: &PlatformVersion,
) -> Result<DataContract, Error> {
    // The protocol caps a contract's bytes in the format state transitions carry, and, where
    // the verifier's protocol version stores contracts compiled, a compiled contract's bytes
    // under their own, larger cap
    let contract_versions = &platform_version.dpp.contract_versions;
    let max_size = if contract_versions.contract_storage_serialization_version
        >= u16::from(COMPILED_CONTRACT_FORMAT_TAG)
        && serialized_contract.first() == Some(&COMPILED_CONTRACT_FORMAT_TAG)
    {
        platform_version
            .system_limits
            .max_compiled_contract_size
            .map_or(MAX_CONTRACT_DECODE_MEMORY_BYTES, |max_compiled_size| {
                max_compiled_size as usize
            })
    } else {
        contract_versions.max_serialized_size as usize
    };
    if serialized_contract.len() > max_size {
        return Err(ProtocolError::PlatformDeserializationError(format!(
            "serialized proof data contract exceeds the limit of {max_size} bytes for its format"
        ))
        .into());
    }

    let config = PLATFORM_BINCODE_CONFIG.with_limit::<MAX_CONTRACT_DECODE_MEMORY_BYTES>();
    let (serialized_format, consumed) = bincode::borrow_decode_from_slice_untrusted::<
        DataContractInSerializationFormat,
        _,
    >(serialized_contract, config)
    .map_err(|e| {
        ProtocolError::PlatformDeserializationError(format!(
            "unable to deserialize proof data contract within its bounded budget: {e}"
        ))
    })?;

    if consumed != serialized_contract.len() {
        return Err(ProtocolError::PlatformDeserializationError(
            "serialized proof data contract contains trailing bytes".to_string(),
        )
        .into());
    }

    DataContract::try_from_platform_versioned(
        serialized_format,
        // Contract semantics were validated before insertion into authenticated
        // Platform state; proof decoding only reconstructs that stored object.
        false,
        &mut vec![],
        platform_version,
    )
    .map_err(Error::from)
}

pub(super) fn decode_vote_reference(
    serialized_reference: &[u8],
) -> Result<ContestedDocumentResourceVoteReferenceStorageForm, Error> {
    if serialized_reference.len() > MAX_VOTE_REFERENCE_DECODE_BYTES {
        return Err(Error::Drive(DriveError::CorruptedSerialization(
            "serialized vote reference exceeds the proof decoding limit".to_string(),
        )));
    }

    let config = PLATFORM_BINCODE_CONFIG.with_limit::<MAX_VOTE_REFERENCE_DECODE_BYTES>();
    let (reference, consumed) = bincode::decode_from_slice_untrusted(serialized_reference, config)
        .map_err(|e| {
            Error::Drive(DriveError::CorruptedSerialization(format!(
                "serialized vote reference is invalid: {e}"
            )))
        })?;

    if consumed != serialized_reference.len() {
        return Err(Error::Drive(DriveError::CorruptedSerialization(
            "serialized vote reference contains trailing bytes".to_string(),
        )));
    }

    Ok(reference)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::platform_serialization::PLATFORM_BINCODE_CONFIG;
    use dpp::serialization::PlatformSerializableWithPlatformVersion;
    use dpp::tests::fixtures::get_data_contract_fixture;

    #[test]
    fn rejects_oversized_nested_vote_reference_lengths_without_panicking() {
        // CousinReference with a compact bincode length prefix declaring a
        // 64 MiB outer path and no corresponding elements.
        let hostile_reference = hex::decode("04fd0000000004000000").expect("test bytes");

        let result = std::panic::catch_unwind(|| decode_vote_reference(&hostile_reference));

        assert!(result.is_ok(), "bounded proof decoding must not panic");
        assert!(result.expect("decode result").is_err());
    }

    #[test]
    fn rejects_trailing_vote_reference_bytes() {
        let reference = ContestedDocumentResourceVoteReferenceStorageForm {
            reference_path_type:
                grovedb::element::reference_path::ReferencePathType::SiblingReference(vec![1]),
            identity_vote_times: 1,
        };
        let config = PLATFORM_BINCODE_CONFIG;
        let mut bytes = bincode::encode_to_vec(reference, config).expect("encode reference");
        bytes.push(0);

        assert!(decode_vote_reference(&bytes).is_err());
    }

    #[test]
    fn proof_contract_decoder_requires_exact_consumption() {
        let platform_version = PlatformVersion::latest();
        let contract = get_data_contract_fixture(None, 0, platform_version.protocol_version);
        let mut bytes = contract
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("serialize contract fixture");
        bytes.push(0);

        assert!(decode_proof_data_contract(&bytes, platform_version).is_err());
    }

    #[test]
    fn should_decode_a_proof_contract_stored_compiled() {
        let platform_version = PlatformVersion::latest();
        let contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let mut bytes = contract
            .serialize_to_storage_bytes(platform_version)
            .expect("serialize contract fixture for storage");
        assert_eq!(bytes[0], COMPILED_CONTRACT_FORMAT_TAG);

        assert_eq!(
            decode_proof_data_contract(&bytes, platform_version)
                .expect("decode the compiled contract"),
            contract
        );
        bytes.push(0);
        assert!(decode_proof_data_contract(&bytes, platform_version).is_err());
    }

    /// A compiled contract's bytes: its format version, then `len` more bytes
    fn compiled_contract_bytes(len: usize) -> Vec<u8> {
        let mut bytes = vec![0; len + 1];
        bytes[0] = COMPILED_CONTRACT_FORMAT_TAG;
        bytes
    }

    fn refused_for_size(result: Result<DataContract, Error>) -> bool {
        result.is_err_and(|error| error.to_string().contains("exceeds the limit"))
    }

    #[test]
    fn should_bound_a_proof_contract_stored_compiled_by_the_compiled_size_cap() {
        let platform_version = PlatformVersion::latest();
        let over_the_wire_cap =
            platform_version.dpp.contract_versions.max_serialized_size as usize + 1;

        // Over the wire cap, a compiled contract is decoded, here refused as malformed
        let result = decode_proof_data_contract(
            &compiled_contract_bytes(over_the_wire_cap),
            platform_version,
        );
        assert!(result.is_err() && !refused_for_size(result));
        // Over the compiled size cap, it is refused for its size
        assert!(refused_for_size(decode_proof_data_contract(
            &compiled_contract_bytes(
                platform_version
                    .system_limits
                    .max_compiled_contract_size
                    .expect("expected the latest version to cap compiled contracts")
                    as usize
            ),
            platform_version,
        )));
    }

    #[test]
    fn should_cap_a_proof_contract_tagged_compiled_at_the_wire_size_before_protocol_version_15() {
        let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
        let over_the_wire_cap =
            platform_version.dpp.contract_versions.max_serialized_size as usize + 1;

        assert!(refused_for_size(decode_proof_data_contract(
            &compiled_contract_bytes(over_the_wire_cap),
            platform_version,
        )));
    }

    #[test]
    fn proof_contract_decoder_enforces_protocol_wire_size() {
        let platform_version = PlatformVersion::latest();
        let bytes =
            vec![0; platform_version.dpp.contract_versions.max_serialized_size as usize + 1];

        assert!(decode_proof_data_contract(&bytes, platform_version).is_err());
    }

    #[test]
    fn should_decode_a_proof_contract_compiled_up_to_the_compiled_size_cap() {
        use dpp::data_contract::schema::DataContractSchemaMethodsV0;

        let platform_version = PlatformVersion::latest();
        let max_compiled_size = platform_version
            .system_limits
            .max_compiled_contract_size
            .expect("expected the latest version to cap compiled contracts")
            as usize;
        let fixture = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let schema = fixture
            .document_schemas()
            .get("indexedDocument")
            .map(|schema| (*schema).clone())
            .expect("expected the fixture's indexed document type");
        let compiled_size = |contract: &DataContract| {
            contract
                .serialize_to_storage_bytes(platform_version)
                .expect("expected the contract to compile")
                .len()
        };
        let with_copies = |copies: usize| {
            let mut contract = fixture.clone();
            for copy in 0..copies {
                contract
                    .set_document_schema(
                        &format!("copy{copy}"),
                        schema.clone(),
                        false,
                        &mut vec![],
                        platform_version,
                    )
                    .expect("expected to add a document type");
            }
            contract
        };
        // As many copies of a document type as fit the cap
        let base = compiled_size(&fixture);
        let per_copy = compiled_size(&with_copies(1)) - base;
        let mut copies = (max_compiled_size - base) / per_copy;
        let mut contract = with_copies(copies);
        while compiled_size(&contract) > max_compiled_size {
            copies -= 1;
            contract = with_copies(copies);
        }
        let bytes = contract
            .serialize_to_storage_bytes(platform_version)
            .expect("expected the contract to compile");
        assert!(bytes.len() > max_compiled_size - 2 * per_copy);

        assert_eq!(
            decode_proof_data_contract(&bytes, platform_version)
                .expect("expected the contract to decode within the decode budget"),
            contract
        );
    }

    /// A document type of `objects` properties, each an integer or, with `fields`, an object of
    /// that many integers, with an index on each of the first `indices`: as many properties and
    /// indexes per byte as a document type holds, the compiled containers the decode budget
    /// charges most for.
    fn dense_document_schema(
        objects: u32,
        fields: u32,
        indices: u32,
    ) -> dpp::platform_value::Value {
        use dpp::platform_value::{platform_value, Value};

        let properties = Value::Map(
            (0..objects)
                .map(|object| {
                    let property = if fields == 0 {
                        platform_value!({ "type": "integer", "position": object })
                    } else {
                        let nested = Value::Map(
                            (0..fields)
                                .map(|field| {
                                    (
                                        Value::Text(format!("f{field}")),
                                        platform_value!({ "type": "integer", "position": field }),
                                    )
                                })
                                .collect(),
                        );
                        platform_value!({
                            "type": "object",
                            "properties": nested,
                            "additionalProperties": false,
                            "position": object,
                        })
                    };
                    (Value::Text(format!("o{object}")), property)
                })
                .collect(),
        );
        let indices = Value::Array(
            (0..indices)
                .map(|index| {
                    let path = if fields == 0 {
                        format!("o{index}")
                    } else {
                        format!("o{index}.f0")
                    };
                    platform_value!({ "name": format!("i{index}"), "properties": [{ path: "asc" }] })
                })
                .collect(),
        );
        platform_value!({
            "type": "object",
            "properties": properties,
            "indices": indices,
            "additionalProperties": false,
        })
    }

    #[test]
    fn should_decode_a_proof_contract_compiled_up_to_the_cap_in_one_dense_document_type() {
        use dpp::data_contract::schema::DataContractSchemaMethodsV0;

        let platform_version = PlatformVersion::latest();
        let max_compiled_size = platform_version
            .system_limits
            .max_compiled_contract_size
            .expect("expected the latest version to cap compiled contracts")
            as usize;
        let fixture = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let with_dense_type = |objects: u32, fields: u32, indices: u32| {
            let mut contract = fixture.clone();
            contract
                .set_document_schema(
                    "dense",
                    dense_document_schema(objects, fields, indices),
                    false,
                    &mut vec![],
                    platform_version,
                )
                .expect("expected to add the dense document type");
            let bytes = contract
                .serialize_to_storage_bytes(platform_version)
                .expect("expected the contract to compile");
            (contract, bytes)
        };
        // Flat integers with indexes, and objects of nested integers
        for (fields, indices) in [(0, 10), (100, 0)] {
            // Nearly as many properties as fit the cap: scaled by the size they compile to, as
            // longer names and positions make later properties larger
            let first = indices.max(1);
            let base = with_dense_type(first, fields, indices).1.len();
            let mut objects = first + 1_000;
            let (contract, bytes) = loop {
                let (contract, bytes) = with_dense_type(objects, fields, indices);
                if bytes.len() <= max_compiled_size && bytes.len() * 20 >= max_compiled_size * 19 {
                    break (contract, bytes);
                }
                objects = first
                    + ((objects - first) as usize * (max_compiled_size - base) * 49
                        / ((bytes.len() - base) * 50)) as u32;
            };
            assert_eq!(
                decode_proof_data_contract(&bytes, platform_version).unwrap_or_else(|error| {
                    panic!("{objects} objects of {fields} fields do not decode: {error}")
                }),
                contract
            );
        }
    }
}
