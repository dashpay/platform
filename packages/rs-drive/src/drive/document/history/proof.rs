//! The two GroveDB proofs of a history page in one envelope.

use super::{corrupt, invalid};
use crate::error::Error;

/// The two GroveDB proofs a history page needs, carried as one proof on the
/// wire.
///
/// They answer two queries GroveDB cannot merge: the offset-paginated read of
/// the document's history tree, and the exact-key absence-proof read of the
/// current pointer, the lifecycle record and the history tree's count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentHistoryProofV1 {
    /// Omitted only if metadata proves that the history tree is absent.
    pub entries_proof: Option<Vec<u8>>,
    /// Proves the current pointer and the raw history count-tree element.
    pub metadata_proof: Vec<u8>,
}

/// Version byte of the only proof envelope layout that exists.
const DOCUMENT_HISTORY_PROOF_ENVELOPE_V0: u8 = 0;

impl DocumentHistoryProofV1 {
    /// Encodes both proofs as one byte string: a version byte, the metadata
    /// proof behind a big-endian `u32` length, then a presence byte for the
    /// entries proof followed, when present, by its length and bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let entries_len = self
            .entries_proof
            .as_ref()
            .map_or(0, |proof| 4 + proof.len());
        let mut bytes = Vec::with_capacity(1 + 4 + self.metadata_proof.len() + 1 + entries_len);
        bytes.push(DOCUMENT_HISTORY_PROOF_ENVELOPE_V0);
        bytes.extend((self.metadata_proof.len() as u32).to_be_bytes());
        bytes.extend(&self.metadata_proof);
        match &self.entries_proof {
            Some(proof) => {
                bytes.push(1);
                bytes.extend((proof.len() as u32).to_be_bytes());
                bytes.extend(proof);
            }
            None => bytes.push(0),
        }
        bytes
    }

    /// Decodes an envelope written by [`Self::to_bytes`], requiring the whole
    /// input to be consumed: a proof is produced only by a node, so anything
    /// else is not the proof this code believes it is reading.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        fn take<'a>(
            bytes: &mut &'a [u8],
            len: usize,
            what: &'static str,
        ) -> Result<&'a [u8], Error> {
            if bytes.len() < len {
                return Err(corrupt(what));
            }
            let (head, tail) = bytes.split_at(len);
            *bytes = tail;
            Ok(head)
        }
        fn length(bytes: &mut &[u8], what: &'static str) -> Result<usize, Error> {
            let mut buffer = [0u8; 4];
            buffer.copy_from_slice(take(bytes, 4, what)?);
            Ok(u32::from_be_bytes(buffer) as usize)
        }
        let mut rest = bytes;
        let version = take(&mut rest, 1, "document history proof envelope is empty")?[0];
        if version != DOCUMENT_HISTORY_PROOF_ENVELOPE_V0 {
            return Err(corrupt("unknown document history proof envelope version"));
        }
        let metadata_len = length(
            &mut rest,
            "document history proof envelope lacks its metadata proof length",
        )?;
        let metadata_proof = take(
            &mut rest,
            metadata_len,
            "document history proof envelope is shorter than its metadata proof",
        )?
        .to_vec();
        let entries_proof = match take(
            &mut rest,
            1,
            "document history proof envelope lacks its entries presence byte",
        )?[0]
        {
            0 => None,
            1 => {
                let entries_len = length(
                    &mut rest,
                    "document history proof envelope lacks its entries proof length",
                )?;
                Some(
                    take(
                        &mut rest,
                        entries_len,
                        "document history proof envelope is shorter than its entries proof",
                    )?
                    .to_vec(),
                )
            }
            _ => {
                return Err(corrupt(
                    "document history proof envelope has an invalid entries presence byte",
                ))
            }
        };
        if !rest.is_empty() {
            return Err(corrupt(
                "document history proof envelope has trailing bytes",
            ));
        }
        Ok(Self {
            entries_proof,
            metadata_proof,
        })
    }
}

#[cfg(any(feature = "server", feature = "verify"))]
impl DocumentHistoryProofV1 {
    /// Requires envelopes that bind terminal tree counts and pagination bounds.
    /// Full decoding and cryptographic verification follow this version check.
    pub fn validate_envelopes(&self) -> Result<(), Error> {
        for bytes in std::iter::once(&self.metadata_proof).chain(self.entries_proof.iter()) {
            // GroveDB serializes its proof enum discriminant as a bincode u32.
            let (envelope, _): (u32, usize) =
                bincode::decode_from_slice(bytes, bincode::config::standard().with_big_endian())
                    .map_err(|_| corrupt("invalid document history proof envelope"))?;
            if envelope != 1 {
                return Err(invalid("unsupported proof version: document history proofs require GroveDB v1 proof envelopes"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod proof_envelope_tests {
    use super::DocumentHistoryProofV1;

    #[test]
    fn should_round_trip_with_and_without_an_entries_proof() {
        for entries_proof in [Some(vec![1u8, 2, 3]), None, Some(vec![])] {
            let proof = DocumentHistoryProofV1 {
                entries_proof,
                metadata_proof: vec![9u8; 5],
            };
            assert_eq!(
                DocumentHistoryProofV1::from_bytes(&proof.to_bytes()).unwrap(),
                proof
            );
        }
    }

    #[test]
    fn should_reject_truncated_altered_and_padded_envelopes() {
        let bytes = DocumentHistoryProofV1 {
            entries_proof: Some(vec![1u8, 2, 3]),
            metadata_proof: vec![9u8; 5],
        }
        .to_bytes();
        for cut in [0, 1, 4, 6, 10, bytes.len() - 1] {
            assert!(
                DocumentHistoryProofV1::from_bytes(&bytes[..cut]).is_err(),
                "cut at {cut}"
            );
        }
        let mut padded = bytes.clone();
        padded.push(0);
        assert!(DocumentHistoryProofV1::from_bytes(&padded).is_err());
        let mut wrong_version = bytes.clone();
        wrong_version[0] = 1;
        assert!(DocumentHistoryProofV1::from_bytes(&wrong_version).is_err());
        let mut bad_presence = bytes;
        bad_presence[1 + 4 + 5] = 2;
        assert!(DocumentHistoryProofV1::from_bytes(&bad_presence).is_err());
    }
}
