use bincode::{Decode, Encode};

pub mod v1;

/// Fees of an entry a `PrivateDocumentCreateFromShieldedPool` appends to a private document
/// type's store (protocol version 15).
///
/// The transition pays a fixed fee out of the credit shielded pool, so it cannot be metered:
/// its value balance must equal the fee exactly, and a wallet computes it before building the
/// bundle. On top of the bundle's shielded fee, each entry pays its own bytes and
/// `entry_overhead_bytes` at the storage rate, plus `entry_processing_fee`.
///
/// Every earlier schedule carries the same group; it is unread there because the transition
/// is refused before protocol version 15.
#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeePrivateDocumentVersion {
    /// The flat credits each entry pays on top of its storage. It prices the scan every wallet
    /// of the type makes of the entry, more than the work of storing it, so a flood of entries
    /// costs what it makes others download.
    pub entry_processing_fee: u64,
    /// The bytes each entry is charged for beyond its own: Drive's 16-byte header, and what
    /// GroveDB's append into the store adds per entry, the compaction of a full chunk shared out
    /// among its entries, for every chunk power a store can take.
    pub entry_overhead_bytes: u64,
}
