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

/// What retiring a round settled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadinessRetirement {
    /// The credits the fund held when the round retired.
    pub fund_balance: Credits,
    /// The credits charged to the epoch's processing pool for the deferred cleanup.
    pub cleanup_charged: Credits,
    /// The credits owed to the payer; the caller settles them (a single balance write per
    /// batch, netted against any new funding).
    pub refund: Credits,
}

impl Drive {
    /// Retires a round the caller has already unlinked from its contract's pointer: queues
    /// it for the bounded cleanup, removes its deadline entry when it had crossed, empties
    /// its fund, charges the cleanup reserve to the epoch's processing pool and reports the
    /// remainder owed to the payer.
    ///
    /// Never opens the reports tree, so the batch is a fixed number of operations whatever
    /// the round accumulated. The refund is returned rather than written because a batch
    /// may both refund the old payer and debit the new funding on the same identity, and
    /// two absolute balance writes on one key in one batch collapse; the caller nets them.
    ///
    /// One retirement per applied batch: the pool credit is an absolute rewrite of the
    /// epoch's processing pool item, so two retirements in one batch would collapse too.
    ///
    /// # Parameters
    ///
    /// * `round` - The round being retired.
    /// * `cleanup_reserve` - The credits charged to the pool for the deferred cleanup, capped
    ///   at what the fund holds.
    /// * `block_info` - The block the retirement happens in (its epoch receives the reserve).
    /// * `estimated_costs_only_with_layer_info` - `Some` to estimate instead of read state.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * What was settled and the low level operations that perform the writes.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    #[allow(clippy::too_many_arguments)]
    pub fn retire_readiness_round_operations(
        &self,
        round: &ReadinessRound,
        cleanup_reserve: Credits,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(ReadinessRetirement, Vec<LowLevelDriveOperation>), Error> {
        match platform_version.drive.methods.vote.readiness.retire_round {
            Some(0) => self.retire_readiness_round_operations_v0(
                round,
                cleanup_reserve,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "retire_readiness_round_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "retire_readiness_round_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
