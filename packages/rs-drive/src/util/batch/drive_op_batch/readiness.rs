use crate::drive::votes::readiness::ReadinessRoundFunding;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use crate::util::batch::DriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::voting::readiness::report_record::ReadinessReportRecord;
use dpp::voting::readiness::round::ReadinessRoundOpening;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

/// Operations on compilation readiness rounds
#[derive(Clone, Debug)]
pub enum ReadinessOperationType {
    /// Opens a contract's round, replacing any current one
    OpenRound {
        /// What identifies the round
        opening: ReadinessRoundOpening,
        /// The credits the opening moves
        funding: ReadinessRoundFunding,
    },
    /// Cancels a contract's current round
    CancelRound {
        /// The contract
        contract_id: [u8; 32],
        /// The credits charged to the pool for the deferred cleanup
        cleanup_reserve: Credits,
    },
    /// Inserts one accepted report into the current round
    InsertReport {
        /// The contract
        contract_id: [u8; 32],
        /// The round; the caller has checked it is the contract's current round
        round_id: [u8; 32],
        /// The reporting evonode
        pro_tx_hash: [u8; 32],
        /// What to store for the report
        record: ReadinessReportRecord,
    },
    /// Deletes named reports from a round
    PruneReports {
        /// The contract
        contract_id: [u8; 32],
        /// The round
        round_id: [u8; 32],
        /// The reports to delete
        pro_tx_hashes: Vec<[u8; 32]>,
    },
}

impl ReadinessOperationType {
    /// Whether the operation can retire a round: an opening retires the contract's previous
    /// round when there is one, a cancellation always does.
    fn can_retire_a_round(&self) -> bool {
        matches!(
            self,
            ReadinessOperationType::OpenRound { .. } | ReadinessOperationType::CancelRound { .. }
        )
    }
}

/// Refuses a batch holding more than one operation that can retire a readiness round.
///
/// A retirement credits its cleanup reserve to the epoch's processing pool as an absolute
/// rewrite of the pool item computed from the value read before the batch applies, so a
/// second retirement in the same batch would overwrite the first credit instead of adding to
/// it. Callers schedule one retirement per batch; this keeps a batch that breaks the rule
/// from losing credits.
pub(crate) fn verify_at_most_one_readiness_retirement(
    operations: &[DriveOperation],
) -> Result<(), Error> {
    let retirements = operations
        .iter()
        .filter(|operation| {
            matches!(
                operation,
                DriveOperation::ReadinessOperation(readiness_operation)
                    if readiness_operation.can_retire_a_round()
            )
        })
        .count();
    if retirements > 1 {
        return Err(Error::Drive(DriveError::NotSupported(
            "one readiness round retirement per batch",
        )));
    }
    Ok(())
}

impl DriveLowLevelOperationConverter for ReadinessOperationType {
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            ReadinessOperationType::OpenRound { opening, funding } => drive
                .open_readiness_round_operations(
                    opening,
                    funding,
                    block_info,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )
                .map(|(_, operations)| operations),
            ReadinessOperationType::CancelRound {
                contract_id,
                cleanup_reserve,
            } => drive
                .cancel_readiness_round_operations(
                    contract_id,
                    cleanup_reserve,
                    block_info,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )
                .map(|(_, operations)| operations),
            ReadinessOperationType::InsertReport {
                contract_id,
                round_id,
                pro_tx_hash,
                record,
            } => drive
                .insert_readiness_report_operations(
                    contract_id,
                    round_id,
                    pro_tx_hash,
                    &record,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )
                .map(|(_, operations)| operations),
            ReadinessOperationType::PruneReports {
                contract_id,
                round_id,
                pro_tx_hashes,
            } => drive.prune_readiness_reports_operations(
                contract_id,
                round_id,
                &pro_tx_hashes,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
        }
    }
}
