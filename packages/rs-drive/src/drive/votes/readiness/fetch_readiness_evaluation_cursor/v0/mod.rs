use crate::drive::votes::paths::{readiness_tree_path, READINESS_EVALUATION_CURSOR_KEY};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};

impl Drive {
    pub(super) fn fetch_readiness_evaluation_cursor_operations_v0(
        &self,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<[u8; 32]>, Error> {
        let path = readiness_tree_path();
        match self.grove_get_raw_optional(
            (&path).into(),
            &[READINESS_EVALUATION_CURSOR_KEY],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )? {
            None => Ok(None),
            Some(Element::Item(bytes, _)) => {
                let contract_id: [u8; 32] = bytes.try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedDriveState(
                        "readiness evaluation cursor is not 32 bytes".to_string(),
                    ))
                })?;
                Ok(Some(contract_id))
            }
            Some(_) => Err(Error::Drive(DriveError::CorruptedElementType(
                "readiness evaluation cursor was present but was not an item",
            ))),
        }
    }
}
