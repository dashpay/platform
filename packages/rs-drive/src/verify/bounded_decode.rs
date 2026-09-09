use crate::drive::votes::storage_form::contested_document_resource_reference_storage_form::ContestedDocumentResourceVoteReferenceStorageForm;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::bincode;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
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
/// Contract input bytes are separately capped by the active protocol
/// version's `max_serialized_size` (currently 65,000 bytes). Bincode charges
/// this budget for decoded containers rather than wire bytes, so the larger
/// value preserves ample compatibility headroom while still bounding hostile
/// compact length prefixes before a proof root is trusted.
const MAX_CONTRACT_DECODE_MEMORY_BYTES: usize = 16 * 1024 * 1024;

pub(super) fn decode_proof_data_contract(
    serialized_contract: &[u8],
    platform_version: &PlatformVersion,
) -> Result<DataContract, Error> {
    let max_serialized_size = platform_version.dpp.contract_versions.max_serialized_size as usize;
    if serialized_contract.len() > max_serialized_size {
        return Err(ProtocolError::PlatformDeserializationError(format!(
            "serialized proof data contract exceeds the protocol limit of {max_serialized_size} bytes"
        ))
        .into());
    }

    let config = bincode::config::standard()
        .with_big_endian()
        .with_limit::<MAX_CONTRACT_DECODE_MEMORY_BYTES>();
    let (serialized_format, consumed) = bincode::borrow_decode_from_slice::<
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

    let config = bincode::config::standard()
        .with_big_endian()
        .with_limit::<MAX_VOTE_REFERENCE_DECODE_BYTES>();
    let (reference, consumed) =
        bincode::decode_from_slice(serialized_reference, config).map_err(|e| {
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
        let config = bincode::config::standard().with_big_endian();
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
    fn proof_contract_decoder_enforces_protocol_wire_size() {
        let platform_version = PlatformVersion::latest();
        let bytes =
            vec![0; platform_version.dpp.contract_versions.max_serialized_size as usize + 1];

        assert!(decode_proof_data_contract(&bytes, platform_version).is_err());
    }
}
