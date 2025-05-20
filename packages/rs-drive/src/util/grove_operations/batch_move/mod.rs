mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchMoveApplyType;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;
use grovedb_epoch_based_storage_flags::StorageFlags;
use grovedb_path::SubtreePath;

impl Drive {
    /// Push a *single‑item* “move” (delete + insert) operation into the batch.
    #[allow(clippy::too_many_arguments)]
    pub fn batch_move<B: AsRef<[u8]>>(
        &self,
        from_path: SubtreePath<'_, B>,
        key: &[u8],
        to_path: Vec<Vec<u8>>,
        apply_type: BatchMoveApplyType,
        alter_flags_to_new_flags: Option<Option<StorageFlags>>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version.grove_methods.batch.batch_move {
            0 => self.batch_move_v0(
                from_path,
                key,
                to_path,
                apply_type,
                alter_flags_to_new_flags,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_move".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
