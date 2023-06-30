mod v0;

use grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee_pools::epochs::paths::EpochProposers;
use dpp::block::epoch::Epoch;
use dpp::version::drive_versions::DriveVersion;

impl Drive {

    /// Returns the given proposer's block count
    ///
    /// # Arguments
    ///
    /// * `epoch` - An Epoch instance.
    /// * `proposer_tx_hash` - An array of bytes containing the transaction hash of the proposer.
    /// * `transaction` - A TransactionArg instance.
    /// * `drive_version` - A DriveVersion instance representing the version of the drive.
    ///
    /// # Returns
    ///
    /// A Result containing the block count or an Error.
    pub fn get_epochs_proposer_block_count(
        &self,
        epoch: &Epoch,
        proposer_tx_hash: &[u8; 32],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<u64, Error> {
        match drive_version.methods.credit_pools.epochs.get_epochs_proposer_block_count {
            0 => self.get_epochs_proposer_block_count_v0(epoch, proposer_tx_hash, transaction),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_epochs_proposer_block_count".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}