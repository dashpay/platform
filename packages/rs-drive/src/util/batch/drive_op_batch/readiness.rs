use crate::drive::votes::readiness::ReadinessRoundFunding;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use crate::util::batch::drive_op_batch::{
    IdentityOperationType, PrefundedSpecializedBalanceOperationType,
};
use crate::util::batch::DriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::voting::readiness::report_record::ReadinessReportRecord;
use dpp::voting::readiness::round::ReadinessRoundOpening;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::{BTreeSet, HashMap};

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
    /// Activates a contract's crossed round at its deadline
    ActivateRound {
        /// The contract
        contract_id: [u8; 32],
        /// The round the deadline entry names
        round_id: [u8; 32],
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
    /// round when there is one, a cancellation and an activation always do.
    fn can_retire_a_round(&self) -> bool {
        matches!(
            self,
            ReadinessOperationType::OpenRound { .. }
                | ReadinessOperationType::CancelRound { .. }
                | ReadinessOperationType::ActivateRound { .. }
        )
    }
}

/// Refuses a batch whose readiness writes would overwrite each other.
///
/// Readiness writes compute their new value from the one committed before the batch, and
/// GroveDB keeps only the last write of a key, so a second write of the same key in one batch
/// would replace the first instead of adding to it. Callers schedule one settlement per batch;
/// these checks keep a batch that breaks the rule from losing or minting credits:
///
/// * at most one operation that can retire a round: each credits its cleanup reserve to the
///   epoch's processing pool as an absolute rewrite of the pool item;
/// * a round opening, cancellation or activation shares its batch with no identity balance
///   write and no readiness fund write: it settles the payers and funds of both rounds with absolute
///   writes, and the payer it refunds is only known once the round is read;
/// * each readiness fund is written at most once, since a deduction's reserve check reads the
///   committed balance too;
/// * a round settlement or readiness fund write shares its batch with no raw GroveDB
///   operation: the guard cannot tell which balances a raw write touches.
pub(crate) fn refuse_conflicting_readiness_writes(
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
    let writes_readiness_balances = retirements > 0
        || operations.iter().any(|operation| {
            matches!(
                operation,
                DriveOperation::PrefundedSpecializedBalanceOperation(
                    PrefundedSpecializedBalanceOperationType::CreateNewReadinessFund { .. }
                        | PrefundedSpecializedBalanceOperationType::DeductFromReadinessFund { .. },
                )
            )
        });
    let mut funds_written = BTreeSet::new();
    for operation in operations {
        match operation {
            DriveOperation::PrefundedSpecializedBalanceOperation(
                PrefundedSpecializedBalanceOperationType::CreateNewReadinessFund {
                    fund_id, ..
                }
                | PrefundedSpecializedBalanceOperationType::DeductFromReadinessFund {
                    fund_id, ..
                },
            ) => {
                if retirements > 0 || !funds_written.insert(*fund_id) {
                    return Err(Error::Drive(DriveError::NotSupported(
                        "a readiness fund is written once per batch, never beside a round settlement",
                    )));
                }
            }
            DriveOperation::IdentityOperation(
                IdentityOperationType::AddToIdentityBalance { .. }
                | IdentityOperationType::RemoveFromIdentityBalance { .. },
            ) if retirements > 0 => {
                return Err(Error::Drive(DriveError::NotSupported(
                    "a readiness round settlement is the only identity balance write of its batch",
                )));
            }
            DriveOperation::GroveDBOperation(_) | DriveOperation::GroveDBOpBatch(_)
                if writes_readiness_balances =>
            {
                return Err(Error::Drive(DriveError::NotSupported(
                    "a readiness round settlement or fund write shares its batch with no raw GroveDB operation",
                )));
            }
            _ => {}
        }
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
            ReadinessOperationType::ActivateRound {
                contract_id,
                round_id,
                cleanup_reserve,
            } => drive
                .activate_readiness_round_operations(
                    contract_id,
                    round_id,
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
