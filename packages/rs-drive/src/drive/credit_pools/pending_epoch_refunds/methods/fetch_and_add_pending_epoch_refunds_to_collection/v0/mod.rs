use crate::drive::credit_pools::pending_epoch_refunds::pending_epoch_refunds_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use crate::drive::fee::get_overflow_error;

use dpp::balances::credits::Creditable;
use dpp::fee::epoch::CreditsPerEpoch;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, TransactionArg};

impl Drive {
    /// Fetches pending epoch refunds and adds them to specified collection
    pub(super) fn fetch_and_add_pending_epoch_refunds_to_collection_v0(
        &self,
        mut refunds_per_epoch: CreditsPerEpoch,
        transaction: TransactionArg,
    ) -> Result<CreditsPerEpoch, Error> {
        if refunds_per_epoch.is_empty() {
            return Ok(refunds_per_epoch);
        }

        let mut query = Query::new();

        for epoch_index in refunds_per_epoch.keys() {
            let epoch_index_key = epoch_index.to_be_bytes().to_vec();

            query.insert_key(epoch_index_key);
        }

        // Query existing pending updates
        let (query_result, _) = self
            .grove
            .query_raw(
                &PathQuery::new_unsized(pending_epoch_refunds_path_vec(), query),
                transaction.is_some(),
                true,
                true,
                QueryResultType::QueryKeyElementPairResultType,
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        // Merge with existing pending updates
        for (epoch_index_key, element) in query_result.to_key_elements() {
            let epoch_index =
                u16::from_be_bytes(epoch_index_key.as_slice().try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(String::from(
                        "epoch index for pending epoch refunds must be u16",
                    )))
                })?);

            let existing_credits = refunds_per_epoch.get_mut(&epoch_index).ok_or(Error::Drive(
                DriveError::CorruptedCodeExecution(
                    "pending epoch refunds should contain fetched epochs",
                ),
            ))?;

            if let Element::SumItem(credits, _) = element {
                *existing_credits = existing_credits
                    .checked_add(credits.to_unsigned())
                    .ok_or_else(|| get_overflow_error("pending epoch refunds credits overflow"))?;
            } else {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "pending epoch refunds credits must be sum items",
                )));
            }
        }

        Ok(refunds_per_epoch)
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::fee::epoch::CreditsPerEpoch;

    use dpp::version::PlatformVersion;

    #[test]
    fn should_fetch_and_merge_pending_updates_v0() {
        let drive = setup_drive_with_initial_state_structure();
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        // Store initial set of pending refunds

        let initial_pending_refunds =
            CreditsPerEpoch::from_iter([(1, 15), (3, 25), (7, 95), (9, 100), (12, 120)]);

        let mut batch = vec![];

        Drive::add_update_pending_epoch_refunds_operations(
            &mut batch,
            initial_pending_refunds,
            &platform_version.drive,
        )
        .expect("should update pending epoch updates");

        drive
            .apply_drive_operations(
                batch,
                true,
                &BlockInfo::default(),
                Some(&transaction),
                platform_version,
            )
            .expect("should apply batch");

        // Fetch and merge

        let new_pending_refunds =
            CreditsPerEpoch::from_iter([(1, 15), (3, 25), (30, 195), (41, 150)]);

        let updated_pending_refunds = drive
            .fetch_and_add_pending_epoch_refunds_to_collection(
                new_pending_refunds,
                Some(&transaction),
                &platform_version.drive,
            )
            .expect("should fetch and merge pending updates");

        let expected_pending_refunds =
            CreditsPerEpoch::from_iter([(1, 30), (3, 50), (30, 195), (41, 150)]);

        assert_eq!(updated_pending_refunds, expected_pending_refunds);
    }
}
