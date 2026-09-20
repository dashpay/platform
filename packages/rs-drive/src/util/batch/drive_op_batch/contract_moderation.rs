use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::config::moderation::{ContractDocumentRemoval, ContractModerationReason};
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

/// Operations on a moderated contract's banlist, suspension list and document removal records.
#[derive(Clone, Debug)]
pub enum ContractModerationOperationType {
    /// Puts an identity on the banlist.
    AddBan {
        /// The moderated contract.
        contract_id: Identifier,
        /// The identity to ban.
        identity_id: Identifier,
        /// Why, stored with the entry.
        reason: ContractModerationReason,
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
        /// Why, stored with the entry.
        reason: ContractModerationReason,
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
    /// Records that a moderator deleted a document. The deletion itself is an ordinary
    /// document operation of the same batch.
    AddDocumentRemoval {
        /// The moderated contract.
        contract_id: Identifier,
        /// The document type the document belonged to.
        document_type_name: String,
        /// The id the document had.
        document_id: Identifier,
        /// Whose it was, who removed it (and pays for the record), why and when.
        removal: ContractDocumentRemoval,
        /// Whether a record of that id is already there, which is then replaced.
        replaces_existing: bool,
    },
    /// Writes nothing: marks the batch it is in as one whose storage removals refund nobody
    /// (`Drive::apply_drive_operations` generation 1). A moderator's document deletion carries
    /// it, so the deleted document's owner gets no storage refund.
    ForfeitStorageRefunds,
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
                reason,
                moderator_id,
            } => drive.add_contract_ban_operations(
                contract_id,
                identity_id,
                &reason,
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
                reason,
                replaces_existing,
                moderator_id,
            } => drive.add_contract_suspension_operations(
                contract_id,
                identity_id,
                until,
                &reason,
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
            ContractModerationOperationType::AddDocumentRemoval {
                contract_id,
                document_type_name,
                document_id,
                removal,
                replaces_existing,
            } => drive.add_contract_document_removal_operations(
                contract_id,
                &document_type_name,
                document_id,
                &removal,
                replaces_existing,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            ContractModerationOperationType::ForfeitStorageRefunds => Ok(vec![]),
        }
    }
}
