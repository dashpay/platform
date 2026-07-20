use crate::drive::votes::storage_form::contested_document_resource_reference_storage_form::ContestedDocumentResourceVoteReferenceStorageForm;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::bincode;

/// Maximum decoded resource budget for a proof-derived vote reference.
///
/// Canonical vote references contain only a short, bounded Drive path. This
/// leaves substantial compatibility headroom while preventing compact length
/// prefixes from requesting attacker-selected allocations.
const MAX_VOTE_REFERENCE_DECODE_BYTES: usize = 64 * 1024;

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
}
