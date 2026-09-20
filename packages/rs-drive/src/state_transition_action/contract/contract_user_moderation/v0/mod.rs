mod transformer;

use crate::drive::contract::DataContractFetchInfo;
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
    /// what a document deletion read when the transition was validated, `None` for an action
    /// on an identity
    pub document_deletion: Option<ContractDocumentDeletionContext>,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}

/// What the validation of a document deletion read, so that Drive deletes the document and
/// writes its removal record without reading again.
#[derive(Debug, Clone)]
pub struct ContractDocumentDeletionContext {
    /// the moderated contract, as fetched: the deletion resolves the document type from it
    pub data_contract_fetch_info: Arc<DataContractFetchInfo>,
    /// the owner of the document as stored
    pub document_owner_id: Identifier,
    /// the time of the block the deletion runs in, recorded as the removal time
    pub removed_at: TimestampMillis,
    /// whether a removal record of the same document id is already stored, which the new one
    /// then replaces
    pub replaces_existing_removal: bool,
}
