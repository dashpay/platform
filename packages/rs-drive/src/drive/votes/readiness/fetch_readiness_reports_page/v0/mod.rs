use crate::drive::votes::paths::readiness_round_reports_tree_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::serialization::PlatformDeserializable;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::report_record::ReadinessReportRecord;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fetch_readiness_reports_page_operations_v0(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        after: Option<[u8; 32]>,
        limit: u16,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<([u8; 32], ReadinessReportRecord)>, Error> {
        if limit == 0 {
            return Ok(vec![]);
        }
        let mut query = Query::new_with_direction(true);
        match after {
            Some(after) => query.insert_range_after(after.to_vec()..),
            None => query.insert_all(),
        }
        let path_query = PathQuery::new(
            readiness_round_reports_tree_path_vec(contract_id, round_id),
            SizedQuery::new(query, Some(limit), None),
        );
        let (elements, _) = match self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(result) => result,
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    grovedb::Error::PathParentLayerNotFound(_)
                        | grovedb::Error::PathKeyNotFound(_)
                        | grovedb::Error::InvalidParentLayerPath(_)
                ) =>
            {
                return Ok(vec![]);
            }
            Err(e) => return Err(e),
        };
        elements
            .to_key_elements()
            .into_iter()
            .map(|(key, element)| {
                let pro_tx_hash: [u8; 32] = key.try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "readiness report key is not 32 bytes".to_string(),
                    ))
                })?;
                match element {
                    Element::Item(bytes, _) => {
                        let record = ReadinessReportRecord::deserialize_from_bytes(&bytes)?;
                        Ok((pro_tx_hash, record))
                    }
                    _ => Err(Error::Drive(DriveError::CorruptedElementType(
                        "readiness report was present but was not an item",
                    ))),
                }
            })
            .collect()
    }
}
