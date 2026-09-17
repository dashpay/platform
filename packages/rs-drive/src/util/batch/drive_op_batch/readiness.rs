use crate::drive::votes::readiness::ReadinessRoundFunding;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
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
