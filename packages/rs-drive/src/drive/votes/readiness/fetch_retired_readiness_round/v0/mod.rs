use crate::drive::votes::paths::readiness_retired_rounds_tree_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};

impl Drive {
    pub(super) fn fetch_retired_readiness_round_operations_v0(
        &self,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<([u8; 32], [u8; 32])>, Error> {
        let mut query = Query::new_with_direction(true);
        query.insert_all();
        let path_query = PathQuery::new(
            readiness_retired_rounds_tree_path_vec(),
            SizedQuery::new(query, Some(1), None),
        );
        let (elements, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            drive_operations,
            &platform_version.drive,
        )?;
        let Some((key, element)) = elements.to_key_elements().into_iter().next() else {
            return Ok(None);
        };
        let round_id: [u8; 32] = key.try_into().map_err(|_| {
            Error::Drive(DriveError::CorruptedDriveState(
                "retired readiness round key is not 32 bytes".to_string(),
            ))
        })?;
        let contract_id: [u8; 32] = match element {
            Element::Item(bytes, _) => bytes.try_into().map_err(|_| {
                Error::Drive(DriveError::CorruptedDriveState(
                    "retired readiness round value is not 32 bytes".to_string(),
                ))
            })?,
            _ => {
                return Err(Error::Drive(DriveError::CorruptedElementType(
                    "retired readiness round entry was present but was not an item",
                )))
            }
        };
        Ok(Some((round_id, contract_id)))
    }
}
