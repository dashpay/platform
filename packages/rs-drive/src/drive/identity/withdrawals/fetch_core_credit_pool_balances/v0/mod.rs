use crate::drive::identity::withdrawals::paths::get_withdrawal_core_credit_pool_balances_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::Credits;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, QueryItem, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;
use std::ops::RangeInclusive;

impl Drive {
    pub(super) fn fetch_core_credit_pool_balances_v0(
        &self,
        core_heights: RangeInclusive<u32>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<u32, Credits>, Error> {
        if core_heights.is_empty() {
            return Ok(BTreeMap::new());
        }

        let path_query = PathQuery::new_unsized(
            get_withdrawal_core_credit_pool_balances_path_vec(),
            Query::new_single_query_item(QueryItem::RangeInclusive(
                core_heights.start().to_be_bytes().to_vec()
                    ..=core_heights.end().to_be_bytes().to_vec(),
            )),
        );

        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        results
            .to_key_elements()
            .into_iter()
            .map(|(key, element)| {
                let core_height = u32::from_be_bytes(key.try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(
                        "core credit pool balance key is not 4 bytes".to_string(),
                    ))
                })?);
                let Element::Item(value, _) = element else {
                    return Err(Error::Drive(DriveError::CorruptedElementType(
                        "core credit pool balance is not an item",
                    )));
                };
                let balance = u64::from_be_bytes(value.try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(
                        "core credit pool balance is not 8 bytes".to_string(),
                    ))
                })?);
                Ok((core_height, balance))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_return_the_recorded_balances_of_a_range_and_the_last_recorded_height() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        assert_eq!(
            drive
                .fetch_last_recorded_core_credit_pool_height(Some(&transaction), platform_version)
                .expect("expected the last height"),
            None
        );

        drive
            .record_core_credit_pool_blocks(
                &[(10, 1_000), (11, 1_100), (12, 1_200)],
                Some(&transaction),
                platform_version,
            )
            .expect("expected to record the blocks");

        assert_eq!(
            drive
                .fetch_core_credit_pool_balances(11..=20, Some(&transaction), platform_version)
                .expect("expected the balances"),
            BTreeMap::from([(11, 1_100), (12, 1_200)])
        );
        assert_eq!(
            drive
                .fetch_core_credit_pool_balances(0..=9, Some(&transaction), platform_version)
                .expect("expected the balances"),
            BTreeMap::new()
        );
        assert_eq!(
            drive
                .fetch_last_recorded_core_credit_pool_height(Some(&transaction), platform_version)
                .expect("expected the last height"),
            Some(12)
        );
    }
}
