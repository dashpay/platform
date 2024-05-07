use std::collections::HashMap;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use platform_version::version::PlatformVersion;
use crate::drive::batch::drive_op_batch::DriveLowLevelOperationConverter;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

/// Operations on Prefunded balances
#[derive(Clone, Debug)]
pub enum PrefundedSpecializedBalanceOperationType {
    /// Adds balance to an identity
    CreateNewPrefundedBalance {
        /// The id of the prefunded balance
        prefunded_balance_id: Identifier,
        /// The added balance
        added_balance: u64,
    },
    /// Adds balance to an identity
    DeductFromPrefundedBalance {
        /// The identity id of the identity
        prefunded_balance_id: Identifier,
        /// The removed balance
        removed_balance: u64,
    },
}

impl DriveLowLevelOperationConverter for PrefundedSpecializedBalanceOperationType {
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
            PrefundedSpecializedBalanceOperationType::CreateNewPrefundedBalance {
                prefunded_balance_id, added_balance
            } => drive.add_new_identity_operations(
                identity,
                is_masternode_identity,
                block_info,
                &mut None,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            PrefundedSpecializedBalanceOperationType::DeductFromPrefundedBalance { prefunded_balance_id, removed_balance } => {}
        }
    }
}