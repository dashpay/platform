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
    /// Activates a crossed round at its deadline: deletes the pointer, retires the round
    /// (queued for cleanup, deadline entry dropped, fund emptied with the cleanup reserve
    /// charged to the pool) and credits the remainder to the payer as unused preparation
    /// funding. Routing the activated bundle into the contract's method tables is the
    /// caller's hook; nothing here touches the contract.
    ///
    /// An estimate reads no state and checks nothing: it prices the activation of the largest
    /// round shape (crossed, funded, with a payer to refund) and returns that placeholder in
    /// place of the stored round.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract.
    /// * `round_id` - The round the deadline entry names; must be the contract's current round.
    /// * `cleanup_reserve` - The credits charged to the pool for the deferred cleanup.
    /// * `block_info` - The block the activation happens in.
    /// * `estimated_costs_only_with_layer_info` - `Some` to estimate instead of read state.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The activated round (the placeholder when estimating) and the low level operations
    ///   that perform the writes.
    /// * `Err(DriveError::CorruptedDriveState)` when the round is not the contract's current
    ///   crossed round.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    #[allow(clippy::too_many_arguments)]
    pub fn activate_readiness_round_operations(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        cleanup_reserve: Credits,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(ReadinessRound, Vec<LowLevelDriveOperation>), Error> {
        match platform_version.drive.methods.vote.readiness.activate_round {
            Some(0) => self.activate_readiness_round_operations_v0(
                contract_id,
                round_id,
                cleanup_reserve,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "activate_readiness_round_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "activate_readiness_round_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
