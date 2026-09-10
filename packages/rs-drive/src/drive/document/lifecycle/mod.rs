//! The lifecycle record of a keep-history document, and the per-type tree that holds it.
//!
//! A keep-history document that has been deleted keeps its retained revisions
//! but loses its current pointer and its index references, so nothing in the
//! primary-key tree distinguishes it from a document that never existed. The
//! record supplies that distinction: it exists exactly while the document is
//! deleted or erasing, names the block time it was deleted at, and — once an
//! erasure has been authorized — the block time that erasure started and the
//! revision it started from.
//!
//! It lives in its own tree under the document type rather than inside the
//! document's history, so every key in the history stays in the revision
//! domain and deleted documents stay enumerable per type.

#[cfg(feature = "server")]
mod fetch;

#[cfg(all(test, feature = "server", feature = "verify"))]
mod tests;

#[cfg(feature = "server")]
pub use fetch::DocumentLifecycleState;

use crate::error::{drive::DriveError, Error};

/// Byte length of an encoded lifecycle record: one version byte and four
/// big-endian `u64`s.
pub const DOCUMENT_LIFECYCLE_RECORD_SIZE: u32 = 33;

/// Version byte of the only record layout that exists.
const DOCUMENT_LIFECYCLE_RECORD_V0: u8 = 0;

/// What the committed state says about one keep-history document that is no
/// longer visible to ordinary reads.
///
/// The encoding is fixed width so that the deleted and erasing forms occupy
/// exactly the same number of bytes: an erase start overwrites the record in
/// place, and an overwrite of a different size would re-price storage and drop
/// the original writer's flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DocumentLifecycleRecordV0 {
    /// Block time the document was deleted at.
    pub deleted_at_ms: u64,
    /// Block time an authorized erasure started at, or zero while none has.
    pub erasing_started_at_ms: u64,
    /// Timestamp component of the newest revision retained when the erasure
    /// started, or zero while none has.
    pub erasing_from_time_ms: u64,
    /// History sequence of the newest revision retained when the erasure
    /// started, or zero while none has.
    pub erasing_from_revision: u64,
}

/// A lifecycle record in whichever layout it was written with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentLifecycleRecord {
    /// The original layout.
    V0(DocumentLifecycleRecordV0),
}

impl DocumentLifecycleRecord {
    /// Builds the record a delete writes: a deletion time and no erasure.
    pub fn deleted_at(deleted_at_ms: u64) -> Self {
        DocumentLifecycleRecord::V0(DocumentLifecycleRecordV0 {
            deleted_at_ms,
            ..Default::default()
        })
    }

    /// Block time the document was deleted at.
    pub fn deleted_at_ms(&self) -> u64 {
        match self {
            DocumentLifecycleRecord::V0(v0) => v0.deleted_at_ms,
        }
    }

    /// Block time an authorized erasure started at, or zero while none has.
    pub fn erasing_started_at_ms(&self) -> u64 {
        match self {
            DocumentLifecycleRecord::V0(v0) => v0.erasing_started_at_ms,
        }
    }

    /// Timestamp component of the newest revision retained at erase start.
    pub fn erasing_from_time_ms(&self) -> u64 {
        match self {
            DocumentLifecycleRecord::V0(v0) => v0.erasing_from_time_ms,
        }
    }

    /// History sequence of the newest revision retained at erase start.
    pub fn erasing_from_revision(&self) -> u64 {
        match self {
            DocumentLifecycleRecord::V0(v0) => v0.erasing_from_revision,
        }
    }

    /// Whether an authorized erasure has already started, which is what lets a
    /// continuation run without any authorization of its own.
    pub fn is_erasing(&self) -> bool {
        self.erasing_started_at_ms() != 0
    }

    /// Returns the record an erase start writes over this one, keeping the
    /// deletion time and recording the erasure it authorizes.
    pub fn starting_erase_at(
        &self,
        erasing_started_at_ms: u64,
        erasing_from_time_ms: u64,
        erasing_from_revision: u64,
    ) -> Self {
        DocumentLifecycleRecord::V0(DocumentLifecycleRecordV0 {
            deleted_at_ms: self.deleted_at_ms(),
            erasing_started_at_ms,
            erasing_from_time_ms,
            erasing_from_revision,
        })
    }

    /// Encodes the record as one version byte followed by four big-endian
    /// `u64`s.
    ///
    /// The layout is fixed width rather than the variable-length integer
    /// encoding used elsewhere, because a record whose erase fields are zero
    /// must occupy exactly as many bytes as one whose erase fields are real
    /// timestamps.
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(DOCUMENT_LIFECYCLE_RECORD_SIZE as usize);
        match self {
            DocumentLifecycleRecord::V0(v0) => {
                bytes.push(DOCUMENT_LIFECYCLE_RECORD_V0);
                bytes.extend(v0.deleted_at_ms.to_be_bytes());
                bytes.extend(v0.erasing_started_at_ms.to_be_bytes());
                bytes.extend(v0.erasing_from_time_ms.to_be_bytes());
                bytes.extend(v0.erasing_from_revision.to_be_bytes());
            }
        }
        bytes
    }

    /// Decodes a record, requiring the whole input to be consumed.
    ///
    /// Trailing bytes are a decode failure rather than something to ignore: a
    /// record is written only by consensus, so anything else at that key means
    /// the state is not what this code believes it is.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, Error> {
        let corrupt = |message: &'static str| {
            Error::Drive(DriveError::CorruptedDriveState(message.to_string()))
        };
        let Some((version, fields)) = bytes.split_first() else {
            return Err(corrupt("document lifecycle record is empty"));
        };
        if *version != DOCUMENT_LIFECYCLE_RECORD_V0 {
            return Err(corrupt("unknown document lifecycle record version"));
        }
        if fields.len() != 32 {
            return Err(corrupt(
                "document lifecycle record does not contain four timestamps",
            ));
        }
        let field = |index: usize| -> u64 {
            let mut buffer = [0u8; 8];
            buffer.copy_from_slice(&fields[index * 8..(index + 1) * 8]);
            u64::from_be_bytes(buffer)
        };
        Ok(DocumentLifecycleRecord::V0(DocumentLifecycleRecordV0 {
            deleted_at_ms: field(0),
            erasing_started_at_ms: field(1),
            erasing_from_time_ms: field(2),
            erasing_from_revision: field(3),
        }))
    }
}

#[cfg(test)]
mod record_tests {
    use super::*;

    #[test]
    fn should_encode_deleted_and_erasing_records_to_the_same_length() {
        let deleted = DocumentLifecycleRecord::deleted_at(1_700_000_000_000);
        let erasing = deleted.starting_erase_at(1_700_000_500_000, 1_699_000_000_000, 42);
        assert_eq!(
            deleted.serialize().len(),
            erasing.serialize().len(),
            "an erase start overwrites the record in place and must not change its size"
        );
        assert_eq!(
            deleted.serialize().len(),
            DOCUMENT_LIFECYCLE_RECORD_SIZE as usize
        );
    }

    #[test]
    fn should_round_trip_every_field() {
        let record = DocumentLifecycleRecord::deleted_at(7).starting_erase_at(8, 9, 10);
        let recovered =
            DocumentLifecycleRecord::deserialize(&record.serialize()).expect("round trip");
        assert_eq!(record, recovered);
        assert!(recovered.is_erasing());
    }

    #[test]
    fn should_report_a_record_without_erase_fields_as_deleted_not_erasing() {
        let record = DocumentLifecycleRecord::deleted_at(7);
        assert!(!record.is_erasing());
        assert_eq!(record.erasing_from_revision(), 0);
    }

    #[test]
    fn should_reject_records_that_do_not_consume_their_whole_input() {
        let mut bytes = DocumentLifecycleRecord::deleted_at(7).serialize();
        bytes.push(0);
        assert!(DocumentLifecycleRecord::deserialize(&bytes).is_err());
        assert!(DocumentLifecycleRecord::deserialize(&[]).is_err());
        assert!(DocumentLifecycleRecord::deserialize(&[9; 33]).is_err());
    }
}
