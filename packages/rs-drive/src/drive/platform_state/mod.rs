mod delete_platform_state_entry;
mod fetch_platform_state_bytes;
mod fetch_platform_state_entries_bytes;
mod fetch_platform_state_recent_bytes;
mod store_platform_state_bytes;
mod store_platform_state_entry_bytes;
mod store_platform_state_recent_bytes;

const PLATFORM_STATE_KEY: &[u8; 11] = b"saved_state";

/// The small companion to [`PLATFORM_STATE_KEY`] under saved structure 0: the
/// fields of the platform state that change on every block. That structure's
/// full record is rewritten only when the masternode lists, validator sets or
/// quorum sets change, so this one carries the block info in between. Both are
/// written in the block's transaction, so a reader always sees a pair that
/// committed together. Structure 1 writes its record every block and does not
/// use this key.
const PLATFORM_STATE_RECENT_KEY: &[u8; 18] = b"saved_state_recent";

/// One stored member of a per-entry collection: its key with the collection's
/// prefix removed, and its bytes.
pub type PlatformStateEntry = (Vec<u8>, Vec<u8>);

/// A collection of the platform state kept as one aux entry per member, next to
/// [`PLATFORM_STATE_KEY`], under the collection's key prefix followed by the
/// member's key. A change to one member is then one small write instead of a
/// rewrite of the whole record, and the collection is read back with one prefix
/// scan. The prefixes end in a slash, so neither matches the record keys above.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformStateEntryKind {
    /// The full masternode list, one entry per masternode keyed by ProTxHash.
    Masternodes,
    /// The validator sets, one entry per quorum keyed by quorum hash.
    ValidatorSets,
}

impl PlatformStateEntryKind {
    /// The aux key prefix every entry of this collection sits under.
    pub const fn key_prefix(self) -> &'static [u8] {
        match self {
            PlatformStateEntryKind::Masternodes => b"saved_state/masternodes/",
            PlatformStateEntryKind::ValidatorSets => b"saved_state/validator_sets/",
        }
    }

    /// The aux key of the member with `key`.
    fn entry_key(self, key: &[u8]) -> Vec<u8> {
        [self.key_prefix(), key].concat()
    }
}
