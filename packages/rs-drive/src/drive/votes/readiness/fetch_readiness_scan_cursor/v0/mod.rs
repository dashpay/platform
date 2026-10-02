use crate::drive::votes::paths::{readiness_round_tree_path, READINESS_ROUND_SCAN_CURSOR_KEY};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::serialization::PlatformDeserializableTrusted;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::scan_cursor::ReadinessScanCursor;
use grovedb::{Element, TransactionArg};

impl Drive {
    pub(super) fn fetch_readiness_scan_cursor_operations_v0(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ReadinessScanCursor>, Error> {
        let path = readiness_round_tree_path(&contract_id, &round_id);
        let element = match self.grove_get_raw_optional(
            (&path).into(),
            &[READINESS_ROUND_SCAN_CURSOR_KEY],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(element) => element,
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    grovedb::Error::PathParentLayerNotFound(_)
                        | grovedb::Error::PathKeyNotFound(_)
                        | grovedb::Error::InvalidParentLayerPath(_)
                ) =>
            {
                None
            }
            Err(e) => return Err(e),
        };
        match element {
            None => Ok(None),
            Some(Element::Item(bytes, _)) => Ok(Some(
                ReadinessScanCursor::deserialize_from_bytes_trusted(&bytes)?,
            )),
            Some(_) => Err(Error::Drive(DriveError::CorruptedElementType(
                "readiness scan cursor was present but was not an item",
            ))),
        }
    }
}
