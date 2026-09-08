use crate::drive::identity::withdrawals::fetch_total_credits_in_platform_a_day_ago::{
    RecordedTotalCredits, DAY_IN_MS,
};
use crate::drive::identity::withdrawals::paths::get_withdrawal_total_credits_history_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::prelude::TimestampMillis;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, QueryItem, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn fetch_total_credits_in_platform_a_day_ago_v0(
        &self,
        time_ms: TimestampMillis,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<RecordedTotalCredits>, Error> {
        let cutoff = time_ms.saturating_sub(DAY_IN_MS);

        // The latest entry recorded at or before a day ago.
        let mut at_least_a_day_old = Query::new_single_query_item(QueryItem::RangeToInclusive(
            ..=cutoff.to_be_bytes().to_vec(),
        ));
        at_least_a_day_old.left_to_right = false;

        self.fetch_first_recorded_total_credits(at_least_a_day_old, transaction, platform_version)
    }

    /// Runs `query` with a limit of one against the total credits history and decodes the
    /// single entry it yields, if any.
    pub(in crate::drive::identity::withdrawals) fn fetch_first_recorded_total_credits(
        &self,
        query: Query,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<RecordedTotalCredits>, Error> {
        let path_query = PathQuery::new(
            get_withdrawal_total_credits_history_path_vec(),
            SizedQuery::new(query, Some(1), None),
        );

        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        match results.to_key_elements().into_iter().next() {
            None => Ok(None),
            Some((key, Element::Item(value, _))) => {
                let time_ms =
                    TimestampMillis::from_be_bytes(key.try_into().map_err(|_: Vec<u8>| {
                        Error::Drive(DriveError::CorruptedDriveState(
                            "total credits history key is not a u64 block time".to_string(),
                        ))
                    })?);
                let total_credits =
                    u64::from_be_bytes(value.try_into().map_err(|_: Vec<u8>| {
                        Error::Drive(DriveError::CorruptedDriveState(
                            "total credits history value is not a u64 amount".to_string(),
                        ))
                    })?);
                Ok(Some(RecordedTotalCredits {
                    time_ms,
                    total_credits,
                }))
            }
            Some(_) => Err(Error::Drive(DriveError::CorruptedElementType(
                "total credits history entry is not an item",
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::identity::withdrawals::fetch_total_credits_in_platform_a_day_ago::{
        RecordedTotalCredits, DAY_IN_MS,
    };
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::version::PlatformVersion;

    const HOUR_IN_MS: u64 = 3_600_000;

    #[test]
    fn should_return_none_when_nothing_was_recorded() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        assert_eq!(
            drive
                .fetch_total_credits_in_platform_a_day_ago(
                    10 * DAY_IN_MS,
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to fetch"),
            None
        );
    }

    #[test]
    fn should_return_the_latest_entry_at_least_a_day_old_and_none_before() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let t0 = 10 * DAY_IN_MS;
        // Three blocks, each adding 1000 credits: t0 holds 1000, an hour later 2000, 25 hours
        // later 3000.
        for time_ms in [t0, t0 + HOUR_IN_MS, t0 + 25 * HOUR_IN_MS] {
            drive
                .add_to_system_credits(1_000, Some(&transaction), platform_version)
                .expect("expected to add credits");
            drive
                .record_total_credits_history(
                    &BlockInfo {
                        time_ms,
                        ..Default::default()
                    },
                    0,
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to record");
        }

        let fetch = |time_ms: u64| {
            drive
                .fetch_total_credits_in_platform_a_day_ago(
                    time_ms,
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to fetch")
        };

        // History younger than a day: nothing qualifies.
        assert_eq!(fetch(t0 + 10 * HOUR_IN_MS), None);
        assert_eq!(fetch(t0 + DAY_IN_MS - 1), None);
        // Exactly a day after the first block it is the reference.
        assert_eq!(
            fetch(t0 + DAY_IN_MS),
            Some(RecordedTotalCredits {
                time_ms: t0,
                total_credits: 1_000
            })
        );
        // Exactly a day after the second block: that block is the latest at least a day old.
        assert_eq!(
            fetch(t0 + 25 * HOUR_IN_MS),
            Some(RecordedTotalCredits {
                time_ms: t0 + HOUR_IN_MS,
                total_credits: 2_000
            })
        );
        // A millisecond earlier the second block is still too young.
        assert_eq!(
            fetch(t0 + 25 * HOUR_IN_MS - 1),
            Some(RecordedTotalCredits {
                time_ms: t0,
                total_credits: 1_000
            })
        );
        // Two days later the third block is the reference.
        assert_eq!(
            fetch(t0 + 49 * HOUR_IN_MS),
            Some(RecordedTotalCredits {
                time_ms: t0 + 25 * HOUR_IN_MS,
                total_credits: 3_000
            })
        );
    }
}
