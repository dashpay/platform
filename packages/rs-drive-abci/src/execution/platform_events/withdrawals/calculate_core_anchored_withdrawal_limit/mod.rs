mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// How many more credits withdrawals pooled now may take out of Core's credit pool: a
    /// stricter copy of Core's own asset unlock limit (`core_credit_pool_unlock_limit`), less
    /// what is queued or broadcast and not completed yet. It reads Core's credit pool balance
    /// at the block's chain locked height, and the highest balance among the window starts
    /// Core may measure an unlock pooled now from: Core's credit pool window (576 blocks, 100
    /// on regtest) back from the chain locked height, up to Core's asset unlock validity
    /// (`withdrawal_constants.core_expiration_blocks`, 48) later, as Core mines an unlock
    /// until that many blocks past the height it is signed at while its window moves on and
    /// older deposits leave it. A balance the scan has not recorded yet is read from Core,
    /// which every node answers alike for a chain locked height.
    ///
    /// # Parameters
    ///
    /// * `block_info`: The block being executed; its Core chain locked height is the newest
    ///   Core block the limit reads.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Credits)`: The credits still available on the Core side.
    /// * `Err(Error)` when the method version is unknown or not active, a system limit it
    ///   reads is not configured, Core cannot be asked, or a read fails.
    pub(in crate::execution) fn calculate_core_anchored_withdrawal_limit(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        match platform_version
            .drive_abci
            .methods
            .withdrawals
            .calculate_core_anchored_withdrawal_limit
        {
            Some(0) => self.calculate_core_anchored_withdrawal_limit_v0(
                block_info,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "calculate_core_anchored_withdrawal_limit".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "calculate_core_anchored_withdrawal_limit".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
