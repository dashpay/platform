use crate::state_transition_action::contract::contract_user_moderation::v0::{
    ContractDocumentDeletionContext, ContractUserModerationTransitionActionV0,
};
use dpp::data_contract::config::moderation::ContractModerationStatus;
use dpp::state_transition::contract_user_moderation_transition::v0::ContractUserModerationTransitionV0;

impl ContractUserModerationTransitionActionV0 {
    /// The action of a borrowed transition, keeping of the target's stored status what Drive
    /// needs: whether it carries a suspension
    pub fn from_borrowed_transition_with_status(
        value: &ContractUserModerationTransitionV0,
        current_status: &ContractModerationStatus,
    ) -> Self {
        let ContractUserModerationTransitionV0 {
            owner_id,
            data_contract_id,
            identity_contract_nonce,
            action,
            user_fee_increase,
            ..
        } = value;
        ContractUserModerationTransitionActionV0 {
            moderator_id: *owner_id,
            data_contract_id: *data_contract_id,
            identity_contract_nonce: *identity_contract_nonce,
            action: action.clone(),
            target_is_suspended: current_status.suspension.is_some(),
            document_deletion: None,
            user_fee_increase: *user_fee_increase,
        }
    }

    /// The action of a borrowed transition that deletes a document, carrying what the
    /// validation read about the contract, the document and its removal record
    pub fn from_borrowed_transition_with_document_deletion(
        value: &ContractUserModerationTransitionV0,
        document_deletion: ContractDocumentDeletionContext,
    ) -> Self {
        let mut action =
            Self::from_borrowed_transition_with_status(value, &ContractModerationStatus::default());
        action.document_deletion = Some(document_deletion);
        action
    }
}
