mod v0;

use crate::drive::credit_pools::paths::pools_vec_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::epoch::EpochIndex;
use crate::fee_pools::epochs::paths;
use dpp::block::epoch::{Epoch, EpochIndex};
use grovedb::query_result_type::QueryResultType::QueryPathKeyElementTrioResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};
use crate::drive::credit_pools::epochs::start_block::StartBlockInfo;

use crate::fee_pools::epochs::epoch_key_constants::{
    KEY_START_BLOCK_CORE_HEIGHT, KEY_START_BLOCK_HEIGHT,
};
use crate::fee_pools::epochs::paths::EpochProposers;
use dpp::version::drive_versions::DriveVersion;

impl Drive {

    /// Returns the index and start block platform and core heights of the first epoch between
    /// the two given.
    ///
    /// # Arguments
    ///
    /// * `from_epoch_index` - An EpochIndex instance representing the starting epoch.
    /// * `to_epoch_index` - An EpochIndex instance representing the ending epoch.
    /// * `transaction` - A TransactionArg instance.
    /// * `drive_version` - A DriveVersion instance representing the version of the drive.
    ///
    /// # Returns
    ///
    /// A Result containing the start block info or an Error.
    pub fn get_first_epoch_start_block_info_between_epochs(
        &self,
        from_epoch_index: EpochIndex,
        to_epoch_index: EpochIndex,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Option<StartBlockInfo>, Error> {
        match drive_version.methods.credit_pools.epochs.get_first_epoch_start_block_info_between_epochs {
            0 => self.get_first_epoch_start_block_info_between_epochs_v0(from_epoch_index, to_epoch_index, transaction),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_first_epoch_start_block_info_between_epochs".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}