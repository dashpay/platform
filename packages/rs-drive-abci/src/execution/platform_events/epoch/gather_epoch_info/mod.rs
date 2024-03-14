mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::platform_types::block_proposal;
use crate::platform_types::epoch_info::v0::EpochInfoV0;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Creates epoch info from the platform state and the block proposal.
    ///
    /// The epoch info includes information about the start and end of an epoch,
    /// the validators for the epoch, and other relevant information.
    ///
    /// # Arguments
    ///
    /// * `block_proposal`: The block proposal for which to retrieve the epoch info.
    /// * `transaction`: A reference to the transaction.
    /// * `platform_version`: The version of the platform.
    ///
    /// # Returns
    ///
    /// * `Result<EpochInfoV0, Error>` - The epoch info as an `EpochInfoV0` on success, or an `Error` on failure.
    pub fn gather_epoch_info(
        &self,
        block_proposal: &block_proposal::v0::BlockProposal,
        transaction: &Transaction,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<EpochInfoV0, Error> {
        match platform_version.drive_abci.methods.epoch.gather_epoch_info {
            0 => self.gather_epoch_info_v0(
                block_proposal,
                transaction,
                platform_state,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "gather_epoch_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
