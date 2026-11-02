use crate::drive::platform_state::{PlatformStateEntry, PlatformStateEntryKind};
use crate::drive::Drive;
use crate::error::Error;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn fetch_platform_state_entries_bytes_v0(
        &self,
        kind: PlatformStateEntryKind,
        transaction: TransactionArg,
    ) -> Result<Vec<PlatformStateEntry>, Error> {
        let prefix_len = kind.key_prefix().len();
        let entries = self
            .grove
            .get_aux_by_key_prefix(kind.key_prefix(), transaction)
            .unwrap()
            .map_err(Error::from)?;

        Ok(entries
            .into_iter()
            .map(|(key, bytes)| (key[prefix_len..].to_vec(), bytes))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::platform_state::PlatformStateEntryKind;
    use crate::util::test_helpers::setup::setup_drive;
    use dpp::version::PlatformVersion;

    /// Entries of one collection come back in key order with the prefix removed;
    /// the other collection and the platform state records stay invisible.
    #[test]
    fn should_list_only_the_collections_entries_without_the_prefix() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        for (key, bytes) in [(b"b", b"2"), (b"a", b"1"), (b"c", b"3")] {
            drive
                .store_platform_state_entry_bytes(
                    PlatformStateEntryKind::Masternodes,
                    key,
                    bytes,
                    Some(&transaction),
                    platform_version,
                )
                .expect("store entry");
        }
        drive
            .store_platform_state_entry_bytes(
                PlatformStateEntryKind::ValidatorSets,
                b"a",
                b"validator set",
                Some(&transaction),
                platform_version,
            )
            .expect("store entry");
        drive
            .store_platform_state_bytes(b"record", Some(&transaction), platform_version)
            .expect("store record");
        drive
            .store_platform_state_recent_bytes(b"recent", Some(&transaction), platform_version)
            .expect("store recent record");

        let entries = drive
            .fetch_platform_state_entries_bytes(
                PlatformStateEntryKind::Masternodes,
                Some(&transaction),
                platform_version,
            )
            .expect("fetch entries");
        assert_eq!(
            entries,
            vec![
                (b"a".to_vec(), b"1".to_vec()),
                (b"b".to_vec(), b"2".to_vec()),
                (b"c".to_vec(), b"3".to_vec()),
            ]
        );

        drive
            .delete_platform_state_entry(
                PlatformStateEntryKind::Masternodes,
                b"b",
                Some(&transaction),
                platform_version,
            )
            .expect("delete entry");
        // Deleting an entry that is not there is not an error.
        drive
            .delete_platform_state_entry(
                PlatformStateEntryKind::Masternodes,
                b"zzz",
                Some(&transaction),
                platform_version,
            )
            .expect("delete missing entry");

        let entries = drive
            .fetch_platform_state_entries_bytes(
                PlatformStateEntryKind::Masternodes,
                Some(&transaction),
                platform_version,
            )
            .expect("fetch entries");
        assert_eq!(
            entries,
            vec![
                (b"a".to_vec(), b"1".to_vec()),
                (b"c".to_vec(), b"3".to_vec())
            ]
        );

        let validator_sets = drive
            .fetch_platform_state_entries_bytes(
                PlatformStateEntryKind::ValidatorSets,
                Some(&transaction),
                platform_version,
            )
            .expect("fetch entries");
        assert_eq!(
            validator_sets,
            vec![(b"a".to_vec(), b"validator set".to_vec())]
        );
    }
}
