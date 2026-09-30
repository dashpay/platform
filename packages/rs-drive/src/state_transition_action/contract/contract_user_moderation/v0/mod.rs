mod transformer;

use crate::drive::contract::DataContractFetchInfo;
use dpp::data_contract::config::moderation::{
    ContractDocumentRemoval, ContractSettledDeletion, ContractWarning,
};
use dpp::document::Document;
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};
use dpp::state_transition::contract_user_moderation_transition::ContractUserModerationAction;
use std::sync::Arc;

/// action v0
#[derive(Debug, Clone)]
pub struct ContractUserModerationTransitionActionV0 {
    /// the moderator that signed
    pub moderator_id: Identifier,
    /// the moderated contract
    pub data_contract_id: Identifier,
    /// the moderator's nonce for the contract, used to prevent replay attacks
    pub identity_contract_nonce: IdentityNonce,
    /// what is done, to whom
    pub action: ContractUserModerationAction,
    /// whether the target carried a suspension when the transition was validated, so that
    /// Drive knows whether a suspend replaces an entry and whether a ban also removes a
    /// suspension, without reading again. The status itself, with its reasons, stays behind.
    pub target_is_suspended: bool,
    /// what a warn read when the transition was validated, `None` for every other action
    pub warning: Option<ContractWarningContext>,
    /// what a document deletion read when the transition was validated, `None` for every
    /// other action
    pub document_deletion: Option<ContractDocumentDeletionContext>,
    /// what a document restore read and decoded when the transition was validated, `None`
    /// for every other action
    pub document_restoration: Option<ContractDocumentRestorationContext>,
    /// what a document field change read and built when the transition was validated,
    /// `None` for every other action
    pub document_change: Option<ContractDocumentChangeContext>,
    /// what the approval of a settled document's deletion read and decided when the transition
    /// was validated, `None` for every other action
    pub settled_deletion: Option<ContractSettledDeletionContext>,
    /// the signer's count of moderation actions on the elected contract since its moderators
    /// pot was last settled, this action included, when the signer is on the contract's seated
    /// team and the action counts (a ban, a suspension, a warning or a document deletion);
    /// `None` otherwise. Read when the transition was validated, so Drive writes it without
    /// reading again
    pub moderation_action_count: Option<u32>,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}

/// What the validation of a warn read, so that Drive writes the identity's warning list entry
/// whole, the new warning last, without reading again.
#[derive(Debug, Clone)]
pub struct ContractWarningContext {
    /// the warnings the identity carried, oldest first; empty when it had no entry
    pub existing_warnings: Vec<ContractWarning>,
    /// the time of the block the warn runs in, recorded with the warning
    pub warned_at: TimestampMillis,
}

/// What the validation of a document deletion read, so that Drive deletes the document and
/// writes its removal record, when its type keeps one, without reading again.
#[derive(Debug, Clone)]
pub struct ContractDocumentDeletionContext {
    /// the moderated contract, as fetched: the deletion resolves the document type from it
    pub data_contract_fetch_info: Arc<DataContractFetchInfo>,
    /// the owner of the document as stored
    pub document_owner_id: Identifier,
    /// the removal record to write, `None` when the document's type keeps none
    /// (`moderatorAbilities.deleteKeepsRecord: false`)
    pub record: Option<ContractDocumentRemovalRecordContext>,
    /// whether the document's owner is refunded its storage
    /// (`moderatorAbilities.deleteRefundsOwner`); it forfeits it otherwise
    pub refunds_owner: bool,
}

/// What the validation of a document deletion read and computed for the removal record it
/// leaves.
#[derive(Debug, Clone)]
pub struct ContractDocumentRemovalRecordContext {
    /// the time of the block the deletion runs in, recorded as the removal time
    pub removed_at: TimestampMillis,
    /// a double SHA-256 of the document as serialized under its type, recorded so that a
    /// restore can be checked against it
    pub document_hash: [u8; 32],
    /// whether the document has a removal record already, from a deletion a moderator
    /// restored: the fresh record then replaces it
    pub replaces_restored_record: bool,
}

/// What the validation of a document restore read and decoded, so that Drive puts the
/// document back and marks its removal record without reading again.
#[derive(Debug, Clone)]
pub struct ContractDocumentRestorationContext {
    /// the moderated contract, as fetched: the restore resolves the document type from it
    pub data_contract_fetch_info: Arc<DataContractFetchInfo>,
    /// the document, decoded from the transition's bytes under its type: what is put back
    pub document: Document,
    /// the document's removal record as it will be stored: the record read, marked restored
    /// by the signer at the block's time
    pub removal: ContractDocumentRemoval,
}

/// What the validation of an approval of a settled document's deletion read and decided, so that
/// Drive writes the approvals, and deletes the document when they meet the rule, without reading
/// again.
#[derive(Debug, Clone)]
pub struct ContractSettledDeletionContext {
    /// the approvals as they will be stored: the open ones with this approval added, or a fresh
    /// record holding this one alone; `deleted_at` is set when they meet the document type's
    /// rule
    pub settled_deletion: ContractSettledDeletion,
    /// whether the document has an approval record already, which this one replaces
    pub replaces_existing: bool,
    /// the deletion of the document, as a moderator's `DeleteDocument` would delete it, when the
    /// approvals meet the rule; `None` while they fall short
    pub deletion: Option<ContractDocumentDeletionContext>,
    /// the moderation action count of every approver of a deletion that runs, the signer
    /// included: each one's count since the moderators pot was last settled, one higher, for
    /// Drive to write. Empty while the approvals fall short, and on a contract stored elected
    /// before the counts existed
    pub approver_action_counts: Vec<(Identifier, u32)>,
}

/// What the validation of a document field change read and built, so that Drive stores the
/// changed document without reading again.
#[derive(Debug, Clone)]
pub struct ContractDocumentChangeContext {
    /// the moderated contract, as fetched: the change resolves the document type from it
    pub data_contract_fetch_info: Arc<DataContractFetchInfo>,
    /// the document as it will be stored: the stored one with the fields the transition sets,
    /// its revision one higher, everything else as it was
    pub document: Document,
}
