use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;

/// What the committed state says about one keep-history document that is no
/// longer visible to ordinary reads.
#[derive(Debug, Clone, Copy, Encode, Decode, DecodeUntrusted, From, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct DocumentLifecycleRecordV0 {
    /// Block time the document was deleted at.
    pub deleted_at_ms: u64,
    /// Revision the document carried when it was deleted.
    pub latest_revision: u64,
    /// Number of revisions the history retained when the document was deleted.
    ///
    /// Together with `latest_revision` this says whether the retained history
    /// was contiguous: an erase removes the newest revisions first, so what
    /// survives afterwards is a prefix of what was retained here, and a
    /// by-revision read stays meaningful exactly when the two are equal.
    pub revision_count: u64,
    /// Block time an authorized erasure started at, or zero while none has.
    pub erasing_started_at_ms: u64,
    /// Timestamp component of the newest revision retained when the erasure
    /// started, or zero while none has.
    pub erasing_from_time_ms: u64,
    /// History sequence of the newest revision retained when the erasure
    /// started, or zero while none has.
    pub erasing_from_revision: u64,
}
