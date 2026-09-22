//! The lifecycle record of a keep-history document.
//!
//! A keep-history document that has been deleted keeps its retained revisions
//! but loses its current pointer and its index references, so nothing in the
//! primary-key tree distinguishes it from a document that never existed. The
//! record supplies that distinction: it exists exactly while the document is
//! deleted or erasing, names the block time it was deleted at, and, once an
//! erasure has been authorized, the block time that erasure started and the
//! revision it started from. Drive stores it beside the document's history and
//! proof verifiers decode it from a proof.

use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_versioning::PlatformVersioned;

pub mod v0;

pub use v0::DocumentLifecycleRecordV0;

/// Largest number of bytes an encoded record can occupy: the format
/// discriminant and six variable-length `u64`s at their widest. Estimates
/// that have to price a record without reading it use this bound.
pub const DOCUMENT_LIFECYCLE_RECORD_MAX_SIZE: u32 = 1 + 6 * 9;

/// A lifecycle record in whichever layout it was written with.
#[derive(
    Debug,
    Clone,
    Copy,
    Encode,
    Decode,
    DecodeUntrusted,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    PlatformSerialize,
    PlatformVersioned,
    From,
    PartialEq,
    Eq,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "$formatVersion")
)]
#[platform_serialize(unversioned)]
pub enum DocumentLifecycleRecord {
    /// The original layout.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(DocumentLifecycleRecordV0),
}

impl DocumentLifecycleRecord {
    /// Builds the record a delete writes: the deletion time, what the deleted
    /// document's revision was and how many revisions its history retained,
    /// and no erasure.
    pub fn deleted_at(deleted_at_ms: u64, latest_revision: u64, revision_count: u64) -> Self {
        DocumentLifecycleRecord::V0(DocumentLifecycleRecordV0 {
            deleted_at_ms,
            latest_revision,
            revision_count,
            ..Default::default()
        })
    }

    /// Block time the document was deleted at.
    pub fn deleted_at_ms(&self) -> u64 {
        match self {
            DocumentLifecycleRecord::V0(v0) => v0.deleted_at_ms,
        }
    }

    /// Revision the document carried when it was deleted.
    pub fn latest_revision(&self) -> u64 {
        match self {
            DocumentLifecycleRecord::V0(v0) => v0.latest_revision,
        }
    }

    /// Number of revisions the history retained when the document was deleted.
    pub fn revision_count(&self) -> u64 {
        match self {
            DocumentLifecycleRecord::V0(v0) => v0.revision_count,
        }
    }

    /// Whether the revisions retained at deletion numbered one through the
    /// deleted revision without a gap, which is what lets a by-revision read
    /// map a revision onto a position in the history.
    ///
    /// A gap can only come from a history written before protocol 15, where
    /// two writes in one block overwrote each other's revision; an erase
    /// removes the newest revisions first and so never opens one.
    pub fn revisions_are_contiguous(&self) -> bool {
        self.latest_revision() == self.revision_count()
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
    ///
    /// Read off the revision the erasure started from rather than the time it
    /// started at: a history sequence is one or more for every real revision,
    /// so zero can only mean the field was never written, while a block time of
    /// zero is a value a clock could in principle produce.
    pub fn is_erasing(&self) -> bool {
        self.erasing_from_revision() != 0
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
            latest_revision: self.latest_revision(),
            revision_count: self.revision_count(),
            erasing_started_at_ms,
            erasing_from_time_ms,
            erasing_from_revision,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};

    #[test]
    fn should_round_trip_every_field() {
        let record = DocumentLifecycleRecord::deleted_at(7, 12, 11).starting_erase_at(8, 9, 10);
        let bytes = record.serialize_to_bytes().expect("serialize");
        let recovered =
            DocumentLifecycleRecord::deserialize_from_bytes_untrusted(&bytes).expect("round trip");
        assert_eq!(record, recovered);
        assert!(recovered.is_erasing());
        assert_eq!(recovered.latest_revision(), 12);
        assert_eq!(recovered.revision_count(), 11);
        assert!(!recovered.revisions_are_contiguous());
        assert!(DocumentLifecycleRecord::deleted_at(7, 11, 11).revisions_are_contiguous());
    }

    /// The bound prices a record nobody has read yet, so it must hold for the
    /// widest values every field can take.
    #[test]
    fn should_never_encode_past_the_size_bound() {
        let widest = DocumentLifecycleRecord::deleted_at(u64::MAX, u64::MAX, u64::MAX)
            .starting_erase_at(u64::MAX, u64::MAX, u64::MAX);
        assert_eq!(
            widest.serialize_to_bytes().expect("serialize").len(),
            DOCUMENT_LIFECYCLE_RECORD_MAX_SIZE as usize
        );
        let narrowest = DocumentLifecycleRecord::deleted_at(0, 0, 0);
        assert!(
            narrowest.serialize_to_bytes().expect("serialize").len()
                <= DOCUMENT_LIFECYCLE_RECORD_MAX_SIZE as usize
        );
    }

    #[test]
    fn should_report_a_record_without_erase_fields_as_deleted_not_erasing() {
        let record = DocumentLifecycleRecord::deleted_at(7, 1, 1);
        assert!(!record.is_erasing());
        assert_eq!(record.erasing_from_revision(), 0);
    }

    /// A block whose time is zero must not make a committed erasure look like
    /// one that never started.
    #[test]
    fn should_report_an_erasure_started_in_a_block_at_time_zero_as_erasing() {
        let record = DocumentLifecycleRecord::deleted_at(0, 1, 1).starting_erase_at(0, 0, 1);
        assert!(record.is_erasing());
    }

    #[test]
    fn should_reject_bytes_that_are_not_a_record() {
        assert!(DocumentLifecycleRecord::deserialize_from_bytes_untrusted(&[]).is_err());
        assert!(DocumentLifecycleRecord::deserialize_from_bytes_untrusted(&[9]).is_err());
    }
}
