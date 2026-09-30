mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::round::{ReadinessRound, ReadinessRoundOpening};
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

/// The credit amounts an opening moves. They are parameters rather than fee table reads so
/// that the storage layer carries no fee policy; the state transition that opens a round
/// passes the schedule's values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadinessRoundFunding {
    /// The credits the payer puts into the new round's fund.
    pub initial_funding: Credits,
    /// The credits charged to the pool when the previous round retires.
    pub cleanup_reserve: Credits,
}

impl Drive {
    /// Opens a contract's compilation readiness round, replacing any current one.
    ///
    /// In one batch: the pointer is written to the new round id, the new round tree is
    /// created with its record and empty reports count tree, the new fund is created, and
    /// when a round was current it is retired (queued for cleanup, its deadline entry
    /// dropped, its fund emptied with the cleanup reserve charged to the pool). The retired
    /// fund's refund goes to the payer that funded it: netted against the new funding into
    /// one balance write when the same payer opens the replacement, credited to the previous
    /// payer beside the new payer's debit otherwise.
    /// Old reports cannot count for the new round: they live under the old round's key,
    /// which the pointer no longer names.
    ///
    /// # Parameters
    ///
    /// * `opening` - What identifies the round.
    /// * `funding` - The credits the opening moves.
    /// * `block_info` - The block the round opens in.
    /// * `estimated_costs_only_with_layer_info` - `Some` to estimate instead of read state.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The new round and the low level operations that perform the writes.
    /// * `Err(IdentityError::IdentityInsufficientBalance)` when the payer cannot fund it.
    /// * `Err(DriveError::CorruptedCodeExecution)` when the opening derives the id of the
    ///   contract's current round.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn open_readiness_round_operations(
        &self,
        opening: ReadinessRoundOpening,
        funding: ReadinessRoundFunding,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(ReadinessRound, Vec<LowLevelDriveOperation>), Error> {
        match platform_version.drive.methods.vote.readiness.open_round {
            Some(0) => self.open_readiness_round_operations_v0(
                opening,
                funding,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "open_readiness_round_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "open_readiness_round_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
