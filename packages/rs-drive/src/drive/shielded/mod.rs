/// Shielded pool paths and constants
#[cfg(any(feature = "server", feature = "verify"))]
pub mod paths;
#[cfg(all(feature = "server", any(test, feature = "structure")))]
pub(crate) mod structure;

/// Estimation costs for shielded pool operations
#[cfg(feature = "server")]
pub(crate) mod estimated_costs;

/// Insert the main shielded credit pool and its eight child subtrees.
/// Shared between the genesis-v12 and upgrade-to-v12 paths so both build a
/// byte-identical `[ShieldedBalances]` subtree (consensus-critical).
#[cfg(feature = "server")]
mod insert_shielded_pool_structure;

/// Insert a note into the shielded pool commitment tree
#[cfg(feature = "server")]
mod insert_note;

/// Insert nullifiers into the permanent tree and per-block sync storage
#[cfg(feature = "server")]
mod insert_nullifiers;

/// Update the shielded pool total balance
#[cfg(feature = "server")]
mod update_total_balance;

/// Record the shielded pool anchor if the commitment tree changed this block
#[cfg(feature = "server")]
mod record_anchor_if_changed;

/// Prune shielded pool anchors older than a given cutoff height
#[cfg(feature = "server")]
mod prune_anchors;

/// Check whether a shielded pool anchor exists
#[cfg(feature = "server")]
mod has_anchor;

/// Check whether a nullifier has already been spent
#[cfg(feature = "server")]
mod has_nullifier;

/// Read the shielded pool total balance
#[cfg(feature = "server")]
mod read_total_balance;

/// Count the notes in the shielded pool commitment tree
#[cfg(feature = "server")]
mod notes_count;

#[cfg(test)]
mod tests {
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;

    /// Every primitive here takes the pool's path as a parameter so a token pool can reuse it, and
    /// the credit pool passes the path each one used to build for itself. Protocol version 13 is
    /// the newest released version selecting these generations, so it is where that has to be shown
    /// rather than argued: the whole set is driven through the dispatchers at 13 and each outcome
    /// is the one the credit pool gave before the path became a parameter.
    #[test]
    fn credit_pool_primitives_give_the_same_outcomes_at_protocol_version_13() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::get(13).expect("protocol version 13 should exist");
        let transaction = drive.grove.start_transaction();
        let mut ops = vec![];

        // An untouched pool holds nothing.
        assert_eq!(
            drive
                .shielded_pool_notes_count(Some(&transaction), &mut ops, platform_version)
                .expect("notes count on an untouched pool"),
            0
        );
        assert_eq!(
            drive
                .read_shielded_pool_total_balance(Some(&transaction), &mut ops, platform_version)
                .expect("balance of an untouched pool"),
            0
        );
        let nullifier = [1u8; 32];
        assert!(!drive
            .has_nullifier(&nullifier, Some(&transaction), &mut ops, platform_version)
            .expect("has_nullifier on an untouched pool"));

        // One note, its nullifier, and a balance.
        let mut batch = crate::drive::Drive::insert_note_op(
            nullifier,
            [2u8; 32],
            [3u8; 32],
            vec![0xEEu8; 216],
            platform_version,
        )
        .expect("the note insertion operation");
        batch.extend(
            drive
                .insert_nullifiers(&[nullifier], platform_version)
                .expect("the nullifier insertion operations"),
        );
        batch.extend(
            crate::drive::Drive::update_total_balance_op(5_000, platform_version)
                .expect("the balance write operation"),
        );
        drive
            .grove_apply_batch(
                LowLevelDriveOperation::grovedb_operations_batch_consume(batch),
                false,
                Some(&transaction),
                &platform_version.drive,
            )
            .expect("applying the pool writes");

        assert_eq!(
            drive
                .shielded_pool_notes_count(Some(&transaction), &mut ops, platform_version)
                .expect("notes count after one note"),
            1
        );
        assert_eq!(
            drive
                .read_shielded_pool_total_balance(Some(&transaction), &mut ops, platform_version)
                .expect("balance after the write"),
            5_000
        );
        assert!(drive
            .has_nullifier(&nullifier, Some(&transaction), &mut ops, platform_version)
            .expect("has_nullifier for a recorded one"));
        assert!(!drive
            .has_nullifier(&[9u8; 32], Some(&transaction), &mut ops, platform_version)
            .expect("has_nullifier for an unrecorded one"));

        // The anchor the note produced is recorded once and found.
        crate::drive::Drive::record_shielded_pool_anchor_if_changed(
            &drive,
            1,
            &transaction,
            platform_version,
        )
        .expect("recording the anchor");
    }
}
