use crate::drive::balances::TOTAL_SYSTEM_CREDITS_STORAGE_KEY;
use crate::drive::identity::withdrawals::paths::get_withdrawal_total_credits_history_path_vec;
use crate::drive::system::misc_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::grove_operations::{BatchDeleteApplyType, DirectQueryType};
use crate::util::object_size_info::PathKeyElementInfo;
use dpp::block::block_info::BlockInfo;
use grovedb::{Element, MaybeTree, PathQuery, QueryItem, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn record_total_credits_history_v0(
        &self,
        block_info: &BlockInfo,
        prune_limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut drive_operations = vec![];

        let total_credits_in_platform = self
            .grove_get_raw_value_u64_from_encoded_var_vec(
                (&misc_path()).into(),
                TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "Credits not found in Platform",
            )))?;

        // Resolve the reference entry for this block before adding to the history, so the
        // prune below can never remove the entry the limit reads this block (the new entry is
        // always younger than the reference, so it is never in the pruned range either).
        let reference = self.fetch_total_credits_in_platform_a_day_ago(
            block_info.time_ms,
            transaction,
            platform_version,
        )?;

        let history_path = get_withdrawal_total_credits_history_path_vec();

        self.batch_insert(
            PathKeyElementInfo::PathKeyElement::<0>((
                history_path.clone(),
                block_info.time_ms.to_be_bytes().to_vec(),
                Element::new_item(total_credits_in_platform.to_be_bytes().to_vec()),
            )),
            &mut drive_operations,
            &platform_version.drive,
        )?;

        if let (Some(reference), 1..) = (reference, prune_limit) {
            let mut path_query = PathQuery::new_single_query_item(
                history_path,
                QueryItem::RangeTo(..reference.time_ms.to_be_bytes().to_vec()),
            );
            path_query.query.limit = Some(prune_limit);

            self.batch_delete_items_in_path_query(
                &path_query,
                true,
                // we know that we are not deleting a subtree
                BatchDeleteApplyType::StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                },
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;
        }

        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            drive_operations,
            &mut vec![],
            &platform_version.drive,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::identity::withdrawals::fetch_total_credits_in_platform_a_day_ago::DAY_IN_MS;
    use crate::drive::identity::withdrawals::paths::get_withdrawal_total_credits_history_path_vec;
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::version::PlatformVersion;
    use grovedb::query_result_type::QueryResultType;
    use grovedb::{Element, PathQuery, Query, SizedQuery, Transaction};

    const HOUR_IN_MS: u64 = 3_600_000;

    fn recorded_entries(drive: &Drive, transaction: &Transaction) -> Vec<(u64, u64)> {
        let mut query = Query::new();
        query.insert_all();
        let path_query = PathQuery::new(
            get_withdrawal_total_credits_history_path_vec(),
            SizedQuery::new(query, None, None),
        );
        let (results, _) = drive
            .grove_get_raw_path_query(
                &path_query,
                Some(transaction),
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &PlatformVersion::latest().drive,
            )
            .expect("expected to query the history");
        results
            .to_key_elements()
            .into_iter()
            .map(|(key, element)| {
                let Element::Item(value, _) = element else {
                    panic!("expected an item");
                };
                (
                    u64::from_be_bytes(key.try_into().expect("8 byte key")),
                    u64::from_be_bytes(value.try_into().expect("8 byte value")),
                )
            })
            .collect()
    }

    fn record(drive: &Drive, time_ms: u64, prune_limit: u16, transaction: &Transaction) {
        drive
            .record_total_credits_history(
                &BlockInfo {
                    time_ms,
                    ..Default::default()
                },
                prune_limit,
                Some(transaction),
                PlatformVersion::latest(),
            )
            .expect("expected to record the total credits");
    }

    #[test]
    fn should_record_the_current_total_credits_keyed_by_block_time() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        drive
            .add_to_system_credits(5_000, Some(&transaction), platform_version)
            .expect("expected to add credits");
        record(&drive, 123_456, 64, &transaction);

        drive
            .add_to_system_credits(2_500, Some(&transaction), platform_version)
            .expect("expected to add credits");
        record(&drive, 123_457, 64, &transaction);

        assert_eq!(
            recorded_entries(&drive, &transaction),
            vec![(123_456, 5_000), (123_457, 7_500)]
        );
    }

    #[test]
    fn should_prune_only_entries_older_than_the_reference_entry() {
        let drive = setup_drive_with_initial_state_structure(None);
        let transaction = drive.grove.start_transaction();

        let t0 = 10 * DAY_IN_MS;
        record(&drive, t0, 64, &transaction);
        record(&drive, t0 + HOUR_IN_MS, 64, &transaction);
        record(&drive, t0 + 2 * HOUR_IN_MS, 64, &transaction);

        // Still within the first day: the oldest entry is the reference, nothing is pruned.
        record(&drive, t0 + 23 * HOUR_IN_MS, 64, &transaction);
        assert_eq!(
            recorded_entries(&drive, &transaction)
                .into_iter()
                .map(|(time_ms, _)| time_ms)
                .collect::<Vec<_>>(),
            vec![
                t0,
                t0 + HOUR_IN_MS,
                t0 + 2 * HOUR_IN_MS,
                t0 + 23 * HOUR_IN_MS
            ]
        );

        // A day and two hours in, the reference is the entry at t0 + 2h; the two older entries go,
        // the reference itself and everything younger stay.
        record(&drive, t0 + 26 * HOUR_IN_MS, 64, &transaction);
        assert_eq!(
            recorded_entries(&drive, &transaction)
                .into_iter()
                .map(|(time_ms, _)| time_ms)
                .collect::<Vec<_>>(),
            vec![
                t0 + 2 * HOUR_IN_MS,
                t0 + 23 * HOUR_IN_MS,
                t0 + 26 * HOUR_IN_MS
            ]
        );
    }

    #[test]
    fn should_bound_pruning_per_block_and_skip_it_when_the_limit_is_zero() {
        let drive = setup_drive_with_initial_state_structure(None);
        let transaction = drive.grove.start_transaction();

        let t0 = 10 * DAY_IN_MS;
        for hour in 0..5 {
            record(&drive, t0 + hour * HOUR_IN_MS, 0, &transaction);
        }

        // Reference is t0 + 4h; four older entries are prunable but the limit is zero.
        record(&drive, t0 + 28 * HOUR_IN_MS, 0, &transaction);
        assert_eq!(recorded_entries(&drive, &transaction).len(), 6);

        // With a limit of one, a single stale entry goes per block.
        record(&drive, t0 + 28 * HOUR_IN_MS + 1, 1, &transaction);
        assert_eq!(
            recorded_entries(&drive, &transaction)
                .first()
                .map(|(time_ms, _)| *time_ms),
            Some(t0 + HOUR_IN_MS)
        );
        assert_eq!(recorded_entries(&drive, &transaction).len(), 6);
    }
}
