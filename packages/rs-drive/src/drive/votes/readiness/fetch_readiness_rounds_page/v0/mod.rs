use crate::drive::votes::paths::readiness_contracts_tree_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};

impl Drive {
    pub(super) fn fetch_readiness_rounds_page_operations_v0(
        &self,
        after: Option<[u8; 32]>,
        limit: u16,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<[u8; 32]>, Error> {
        if limit == 0 {
            return Ok(vec![]);
        }
        let mut query = Query::new_with_direction(true);
        match after {
            Some(after) => query.insert_range_after(after.to_vec()..),
            None => query.insert_all(),
        }
        let path_query = PathQuery::new(
            readiness_contracts_tree_path_vec(),
            SizedQuery::new(query, Some(limit), None),
        );
        let (elements, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            drive_operations,
            &platform_version.drive,
        )?;
        elements
            .to_keys()
            .into_iter()
            .map(|key| {
                key.try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "readiness contract key is not 32 bytes".to_string(),
                    ))
                })
            })
            .collect()
    }
}
