mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::platform_types::block_proposal;
use crate::platform_types::epoch_info::EpochInfo;
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
    ) -> Result<EpochInfo, Error> {
        // !! BE AWARE BEFORE YOU MODIFY THIS CODE !!!
        // Please be aware epoch information is gathered with previous platform version
        // on epoch change (1st block of the epoch), despite we are switching to a new version
        // in this block. Thus, the previous version of EpochInfo might also be used for the first block.
        // A new version of this method will be called for the rest of epoch blocks
        // and first block of the next epoch.
        // This means that if we ever want to update EpochInfo, we will need to do so on a release
        // where the new fields of epoch info are not being used. Then make another version once
        // that one is activated.
        match platform_version.drive_abci.methods.epoch.gather_epoch_info {
            0 => self
                .gather_epoch_info_v0(
                    block_proposal,
                    transaction,
                    platform_state,
                    platform_version,
                )
                .map(|v0| v0.into()),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "gather_epoch_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
