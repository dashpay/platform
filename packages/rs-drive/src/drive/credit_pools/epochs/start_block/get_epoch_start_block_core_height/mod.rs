mod v0;

use crate::drive::credit_pools::pools_vec_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::epoch::EpochIndex;
use crate::fee_pools::epochs::paths;
use dpp::block::epoch::Epoch;
use grovedb::query_result_type::QueryResultType::QueryPathKeyElementTrioResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};

use crate::fee_pools::epochs::epoch_key_constants::{
    KEY_START_BLOCK_CORE_HEIGHT, KEY_START_BLOCK_HEIGHT,
};
use crate::fee_pools::epochs::paths::EpochProposers;
use dpp::version::drive_versions::DriveVersion;

impl Drive {
    /// Returns the core block height of the Epoch's start block
    ///
    /// # Arguments
    ///
    /// * `epoch_tree` - An Epoch instance.
    /// * `transaction` - A TransactionArg instance.
    /// * `drive_version` - A DriveVersion instance representing the version of the drive.
    ///
    /// # Returns
    ///
    /// A Result containing the start core block height or an Error.
    pub fn get_epoch_start_block_core_height(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<u32, Error> {
        match drive_version.methods.credit_pools.epochs.get_epoch_start_block_core_height {
            0 => self.get_epoch_start_block_core_height_v0(epoch_tree, transaction),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_epoch_start_block_core_height".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}