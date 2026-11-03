use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::config::moderation::{
    ContractDocumentRemoval, ContractModerationReason, ContractWarning,
};
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

/// Operations on a moderated contract's banlist, suspension list, warning list, document
/// removal records and, for an elected contract, its team's moderation action counts.
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
    /// Writes an identity's warning list entry with one warning more, replacing the entry it
    /// already has.
    AddWarning {
        /// The moderated contract.
        contract_id: Identifier,
        /// The identity to warn.
        identity_id: Identifier,
        /// The identity's warnings after this one, oldest first: what the entry holds.
        warnings: Vec<ContractWarning>,
        /// Whether the identity already has an entry, which is then replaced.
        replaces_existing: bool,
        /// The identity that pays for the entry and receives its refund.
        moderator_id: Identifier,
    },
    /// Takes an identity off the warning list: every warning it carries goes.
    RemoveWarnings {
        /// The moderated contract.
        contract_id: Identifier,
        /// The identity whose warnings are cleared.
        identity_id: Identifier,
    },
    /// Writes the record of a moderator's deletion of a document: a fresh one, or the
    /// replacement of the record the document already has. A record is replaced when a
    /// moderator restores the document (the record then carries the restoration, the
    /// deletion itself undone by a document operation of the same batch) and when a restored
    /// document is deleted again (a fresh record, in place of the restored one). A document id
    /// is produced at most once, so those are the only ways a record can exist already.
    AddDocumentRemoval {
        /// The moderated contract.
        contract_id: Identifier,
        /// The document type the document belonged to.
        document_type_name: String,
        /// The id the document had.
        document_id: Identifier,
        /// Whose it was, who removed it, why and when, what it was, and whether it was
        /// restored since.
        removal: ContractDocumentRemoval,
        /// Whether the document already has a record, which is then replaced.
        replaces_existing: bool,
        /// The identity that pays for the record, or for the bytes a replacement adds, and
        /// receives its refund: the moderator that removed the document, or the one that
        /// restored it.
        moderator_id: Identifier,
    },
    /// Writes nothing: marks the batch it is in as one whose storage removals refund nobody
    /// (`Drive::apply_drive_operations` generation 1). A moderator's document deletion carries
    /// it, so the deleted document's owner gets no storage refund.
    ForfeitStorageRefunds,
    /// Writes a seated moderation team member's count of moderation actions on an elected
    /// contract since the moderators pot was last settled.
    SetActionCount {
        /// The elected contract.
        contract_id: Identifier,
        /// The member that signed the action.
        identity_id: Identifier,
        /// The count to store, the action included.
        count: u32,
    },
    /// Deletes moderation action counts of an elected contract: the reset of a settle of its
    /// moderators pot. Each count must exist.
    RemoveActionCounts {
        /// The elected contract.
        contract_id: Identifier,
        /// The members whose counts go.
        identity_ids: Vec<Identifier>,
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
            ContractModerationOperationType::AddWarning {
                contract_id,
                identity_id,
                warnings,
                replaces_existing,
                moderator_id,
            } => drive.add_contract_warning_operations(
                contract_id,
                identity_id,
                &warnings,
                replaces_existing,
                moderator_id,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            ContractModerationOperationType::RemoveWarnings {
                contract_id,
                identity_id,
            } => drive.remove_contract_warnings_operations(
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
                moderator_id,
            } => drive.add_contract_document_removal_operations(
                contract_id,
                &document_type_name,
                document_id,
                &removal,
                replaces_existing,
                moderator_id,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            ContractModerationOperationType::ForfeitStorageRefunds => Ok(vec![]),
            ContractModerationOperationType::SetActionCount {
                contract_id,
                identity_id,
                count,
            } => drive.set_contract_moderation_action_count_operations(
                contract_id,
                identity_id,
                count,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            ContractModerationOperationType::RemoveActionCounts {
                contract_id,
                identity_ids,
            } => drive.remove_contract_moderation_action_counts_operations(
                contract_id,
                &identity_ids,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
        }
    }
}
