mod delete_platform_state_entry;
mod fetch_platform_state_bytes;
mod fetch_platform_state_entries_bytes;
mod fetch_platform_state_recent_bytes;
mod fetch_reduced_platform_state_bytes;
mod store_platform_state_bytes;
mod store_platform_state_entry_bytes;
mod store_platform_state_recent_bytes;
mod store_reduced_platform_state_bytes;

const PLATFORM_STATE_KEY: &[u8; 11] = b"saved_state";
const REDUCED_PLATFORM_STATE_KEY: &[u8; 19] = b"reduced_saved_state";

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

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use platform_version::version::PlatformVersion;

    #[test]
    fn should_return_none_when_reduced_platform_state_is_absent() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let fetched = drive
            .fetch_reduced_platform_state_bytes(None, platform_version)
            .expect("fetching an absent reduced platform state should not error");

        assert_eq!(fetched, None);
    }

    #[test]
    fn should_roundtrip_reduced_platform_state_bytes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let state_bytes = vec![1u8, 2, 3, 4, 5];

        drive
            .store_reduced_platform_state_bytes(&state_bytes, None, platform_version)
            .expect("should store reduced platform state");

        let fetched = drive
            .fetch_reduced_platform_state_bytes(None, platform_version)
            .expect("should fetch reduced platform state");

        assert_eq!(fetched, Some(state_bytes));
    }

    #[test]
    fn should_overwrite_reduced_platform_state_bytes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        drive
            .store_reduced_platform_state_bytes(&[1u8, 2, 3], None, platform_version)
            .expect("should store reduced platform state");

        let updated_bytes = vec![9u8, 8, 7];
        drive
            .store_reduced_platform_state_bytes(&updated_bytes, None, platform_version)
            .expect("should overwrite reduced platform state");

        let fetched = drive
            .fetch_reduced_platform_state_bytes(None, platform_version)
            .expect("should fetch reduced platform state");

        assert_eq!(fetched, Some(updated_bytes));
    }
}
