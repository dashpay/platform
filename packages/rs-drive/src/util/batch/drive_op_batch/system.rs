use crate::drive::identity::withdrawals::paths::get_withdrawal_credit_inflows_sum_tree_path_vec;
use crate::drive::identity::withdrawals::DAY_AND_A_HOUR_IN_MS;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use crate::util::grove_operations::{BatchInsertApplyType, QueryTarget};
use crate::util::object_size_info::PathKeyElementInfo;
use dpp::block::block_info::BlockInfo;
use dpp::fee::{Credits, SignedCredits};
use dpp::platform_value::Bytes36;

use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

/// Operations on the System
#[derive(Clone, Debug)]
pub enum SystemOperationType {
    /// We want to add credits to the system.
    AddToSystemCredits {
        /// The amount of credits we are seeking to add
        amount: Credits,
    },
    /// We want to remove credits from the system.
    RemoveFromSystemCredits {
        /// The amount of credits we are seeking to remove
        amount: Credits,
    },
    /// Adding a used asset lock, if it is only partially used the asset_lock_value
    /// will have a non 0 remaining_credit_value
    AddUsedAssetLock {
        /// The asset lock outpoint that should be added
        asset_lock_outpoint: Bytes36,
        /// The asset lock value, both initial and remaining
        asset_lock_value: AssetLockValue,
    },
}

impl DriveLowLevelOperationConverter for SystemOperationType {
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
            SystemOperationType::AddToSystemCredits { amount } => {
                let mut drive_operations = drive.add_to_system_credits_operations(
                    amount,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                )?;

                // Every credit mint is also a credit inflow the daily withdrawal limit adds to
                // its daily maximum, so credits that entered Platform within the window may
                // leave again without consuming the withdrawal budget of other users (the limit
                // counts net outflow, not gross).
                match platform_version
                    .drive
                    .methods
                    .identity
                    .withdrawals
                    .record_credit_inflows
                {
                    None => {}
                    Some(0) => {
                        if amount > 0 {
                            let apply_type = if let Some(estimated_costs_only_with_layer_info) =
                                estimated_costs_only_with_layer_info
                            {
                                Drive::add_estimation_costs_for_withdrawal_credit_inflows_update(
                                    estimated_costs_only_with_layer_info,
                                );
                                BatchInsertApplyType::StatelessBatchInsert {
                                    in_tree_type: TreeType::SumTree,
                                    target: QueryTarget::QueryTargetValue(8),
                                }
                            } else {
                                BatchInsertApplyType::StatefulBatchInsert
                            };

                            // Keyed by when the inflow stops counting toward the limit, on the
                            // same 25 hour schedule the withdrawal reservations expire on, and
                            // pruned by the same per-block cleanup. Saturating: a block time
                            // near the end of u64 must not panic, and an entry that never
                            // expires only ever raises the limit toward the gross cap.
                            let expiration_date =
                                block_info.time_ms.saturating_add(DAY_AND_A_HOUR_IN_MS);

                            drive.batch_insert_sum_item_or_add_to_if_already_exists(
                                PathKeyElementInfo::PathKeyElement::<0>((
                                    get_withdrawal_credit_inflows_sum_tree_path_vec(),
                                    expiration_date.to_be_bytes().to_vec(),
                                    Element::SumItem(amount as SignedCredits, None),
                                )),
                                apply_type,
                                transaction,
                                &mut drive_operations,
                                &platform_version.drive,
                            )?;
                        }
                    }
                    Some(version) => {
                        return Err(Error::Drive(DriveError::UnknownVersionMismatch {
                            method: "record_credit_inflows".to_string(),
                            known_versions: vec![0],
                            received: version,
                        }))
                    }
                }

                Ok(drive_operations)
            }
            SystemOperationType::RemoveFromSystemCredits { amount } => drive
                .remove_from_system_credits_operations(
                    amount,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    platform_version,
                ),
            SystemOperationType::AddUsedAssetLock {
                asset_lock_outpoint,
                asset_lock_value,
            } => drive.add_asset_lock_outpoint_operations(
                &asset_lock_outpoint,
                asset_lock_value,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
        }
    }
}
