use crate::drive::identity::withdrawals::paths::get_withdrawal_core_credit_pool_balances_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn fetch_last_recorded_core_credit_pool_height_v0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<u32>, Error> {
        let mut query = Query::new();
        query.insert_all();
        query.left_to_right = false;

        let path_query = PathQuery::new(
            get_withdrawal_core_credit_pool_balances_path_vec(),
            SizedQuery::new(query, Some(1), None),
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
            .next()
            .map(|(key, _)| {
                key.try_into().map(u32::from_be_bytes).map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(
                        "core credit pool balance key is not 4 bytes".to_string(),
                    ))
                })
            })
            .transpose()
    }
}
