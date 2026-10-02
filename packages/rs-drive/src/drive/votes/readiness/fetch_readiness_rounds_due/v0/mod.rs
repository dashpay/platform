use crate::drive::votes::paths::readiness_deadlines_tree_path_vec;
use crate::drive::votes::readiness::fetch_readiness_rounds_due::ReadinessRoundDue;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::common::encode::{decode_u64, encode_u64};
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};

impl Drive {
    pub(super) fn fetch_readiness_rounds_due_operations_v0(
        &self,
        at_ms: u64,
        limit: u16,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<ReadinessRoundDue>, Error> {
        if limit == 0 {
            return Ok(vec![]);
        }
        // Same shape as the vote poll end-date queue: a range over the time keys with a
        // subquery over every entry beneath each of them.
        let mut query = Query::new_with_direction(true);
        query.insert_range_to_inclusive(..=encode_u64(at_ms));
        let mut sub_query = Query::new();
        sub_query.insert_all();
        query.default_subquery_branch.subquery = Some(sub_query.into());
        let path_query = PathQuery {
            path: readiness_deadlines_tree_path_vec(),
            query: SizedQuery {
                query,
                limit: Some(limit),
                offset: None,
            },
        };
        let (elements, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryPathKeyElementTrioResultType,
            drive_operations,
            &platform_version.drive,
        )?;
        elements
            .to_path_key_elements()
            .into_iter()
            .map(|(path, key, element)| {
                let time_key = path
                    .last()
                    .ok_or(Error::Drive(DriveError::CorruptedDriveState(
                        "readiness deadline entry has no time key".to_string(),
                    )))?;
                let deadline_ms = decode_u64(time_key)?;
                let contract_id: [u8; 32] = key.try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "readiness deadline entry key is not 32 bytes".to_string(),
                    ))
                })?;
                let round_id: [u8; 32] = match element {
                    Element::Item(bytes, _) => bytes.try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedDriveState(
                            "readiness deadline entry value is not 32 bytes".to_string(),
                        ))
                    })?,
                    _ => {
                        return Err(Error::Drive(DriveError::CorruptedElementType(
                            "readiness deadline entry was present but was not an item",
                        )))
                    }
                };
                Ok(ReadinessRoundDue {
                    deadline_ms,
                    contract_id,
                    round_id,
                })
            })
            .collect()
    }
}
