mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::round::ReadinessRound;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Cancels a contract's current compilation readiness round: deletes the pointer,
    /// retires the round (queued for cleanup, deadline entry dropped, fund emptied with the
    /// cleanup reserve charged to the pool) and credits the remainder to the payer.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract.
    /// * `cleanup_reserve` - The credits charged to the pool for the deferred cleanup.
    /// * `block_info` - The block the cancellation happens in.
    /// * `estimated_costs_only_with_layer_info` - `Some` to estimate instead of read state.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The cancelled round (`None` when the contract had none, with no operations) and the
    ///   low level operations that perform the writes.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn cancel_readiness_round_operations(
        &self,
        contract_id: [u8; 32],
        cleanup_reserve: Credits,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<ReadinessRound>, Vec<LowLevelDriveOperation>), Error> {
        match platform_version.drive.methods.vote.readiness.cancel_round {
            Some(0) => self.cancel_readiness_round_operations_v0(
                contract_id,
                cleanup_reserve,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "cancel_readiness_round_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "cancel_readiness_round_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
