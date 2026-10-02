use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

/// Operations on Prefunded balances
#[derive(Clone, Debug)]
pub enum PrefundedSpecializedBalanceOperationType {
    /// Creates a new prefunded balance
    CreateNewPrefundedBalance {
        /// The id of the prefunded balance
        prefunded_specialized_balance_id: Identifier,
        /// The added balance
        add_balance: u64,
    },
    /// Deducts from a prefunded balance
    DeductFromPrefundedBalance {
        /// The identity id of the identity
        prefunded_specialized_balance_id: Identifier,
        /// The removed balance
        remove_balance: u64,
    },
    /// Creates a compilation readiness fund or adds to it
    CreateNewReadinessFund {
        /// The fund id, derived from the round id
        fund_id: Identifier,
        /// The added balance
        add_balance: u64,
    },
    /// Deducts from a compilation readiness fund, keeping the cleanup reserve untouchable
    DeductFromReadinessFund {
        /// The fund id, derived from the round id
        fund_id: Identifier,
        /// The removed balance
        remove_balance: u64,
        /// The balance that must remain after the deduction
        reserve: u64,
    },
}

impl DriveLowLevelOperationConverter for PrefundedSpecializedBalanceOperationType {
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            PrefundedSpecializedBalanceOperationType::CreateNewPrefundedBalance {
                prefunded_specialized_balance_id: specialized_balance_id,
                add_balance: added_balance,
            } => drive.add_prefunded_specialized_balance_operations(
                specialized_balance_id,
                added_balance,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            PrefundedSpecializedBalanceOperationType::DeductFromPrefundedBalance {
                prefunded_specialized_balance_id: specialized_balance_id,
                remove_balance: removed_balance,
            } => drive.deduct_from_prefunded_specialized_balance_operations(
                specialized_balance_id,
                removed_balance,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            PrefundedSpecializedBalanceOperationType::CreateNewReadinessFund {
                fund_id,
                add_balance,
            } => drive.add_readiness_fund_operations(
                fund_id,
                add_balance,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            PrefundedSpecializedBalanceOperationType::DeductFromReadinessFund {
                fund_id,
                remove_balance,
                reserve,
            } => drive.deduct_from_readiness_fund_operations(
                fund_id,
                remove_balance,
                reserve,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
        }
    }
}
