//! The one selector a history query carries.

/// Exactly one lower bound or revision position for a history page.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DocumentHistoryFilter {
    /// Inclusive time bound for the first page.
    StartAtTime(u64),
    /// Exclusive composite cursor for subsequent pages.
    StartAfter {
        /// Timestamp of the last entry received.
        time_ms: u64,
        /// History sequence of the last entry received.
        revision: u64,
    },
    /// Inclusive retained revision position.
    StartAtRevision(u64),
    /// One retained revision position.
    Revision(u64),
}
