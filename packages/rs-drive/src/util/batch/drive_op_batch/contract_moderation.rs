use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

/// Operations on a moderated contract's banlist and suspension list.
#[derive(Clone, Debug)]
pub enum ContractModerationOperationType {
    /// Puts an identity on the banlist.
    AddBan {
        /// The moderated contract.
        contract_id: Identifier,
        /// The identity to ban.
        identity_id: Identifier,
        /// The identity that pays for the entry and receives its refund.
        moderator_id: Identifier,
    },
    /// Takes an identity off the banlist.
    RemoveBan {
        /// The moderated contract.
        contract_id: Identifier,
        /// The identity to unban.
        identity_id: Identifier,
    },
    /// Puts an identity on the suspension list until a block time, replacing an entry it
    /// already has.
    AddSuspension {
        /// The moderated contract.
        contract_id: Identifier,
        /// The identity to suspend.
        identity_id: Identifier,
        /// The block time, in milliseconds, at which the suspension lapses.
        until: TimestampMillis,
        /// Whether the identity already has an entry, which is then replaced.
        replaces_existing: bool,
        /// The identity that pays for the entry and receives its refund.
        moderator_id: Identifier,
    },
    /// Takes an identity off the suspension list, lapsed or not.
    RemoveSuspension {
        /// The moderated contract.
        contract_id: Identifier,
        /// The identity to unsuspend.
        identity_id: Identifier,
    },
}

impl DriveLowLevelOperationConverter for ContractModerationOperationType {
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
            ContractModerationOperationType::AddBan {
                contract_id,
                identity_id,
                moderator_id,
            } => drive.add_contract_ban_operations(
                contract_id,
                identity_id,
                moderator_id,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            ContractModerationOperationType::RemoveBan {
                contract_id,
                identity_id,
            } => drive.remove_contract_ban_operations(
                contract_id,
                identity_id,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            ContractModerationOperationType::AddSuspension {
                contract_id,
                identity_id,
                until,
                replaces_existing,
                moderator_id,
            } => drive.add_contract_suspension_operations(
                contract_id,
                identity_id,
                until,
                replaces_existing,
                moderator_id,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            ContractModerationOperationType::RemoveSuspension {
                contract_id,
                identity_id,
            } => drive.remove_contract_suspension_operations(
                contract_id,
                identity_id,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
        }
    }
}
