use crate::state_transition_action::contract::contract_user_moderation::v0::{
    ContractDocumentDeletionContext, ContractDocumentRestorationContext,
    ContractUserModerationTransitionActionV0, ContractWarningContext,
};
use dpp::data_contract::config::moderation::ContractModerationStatus;
use dpp::identity::TimestampMillis;
use dpp::state_transition::contract_user_moderation_transition::v0::ContractUserModerationTransitionV0;
use dpp::state_transition::contract_user_moderation_transition::ContractUserModerationAction;

impl ContractUserModerationTransitionActionV0 {
    /// The action of a borrowed transition, keeping of the target's stored status what Drive
    /// needs: whether it carries a suspension, and for a warn the warnings it carries, which
    /// the entry is rewritten with, and `block_time_ms`, which the new warning is stamped with
    pub fn from_borrowed_transition_with_status(
        value: &ContractUserModerationTransitionV0,
        current_status: &ContractModerationStatus,
        block_time_ms: TimestampMillis,
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
            warning: matches!(action, ContractUserModerationAction::Warn { .. }).then(|| {
                ContractWarningContext {
                    existing_warnings: current_status.warnings.clone(),
                    warned_at: block_time_ms,
                }
            }),
            document_deletion: None,
            document_restoration: None,
            moderation_action_count: None,
            user_fee_increase: *user_fee_increase,
        }
    }

    /// The action of a borrowed transition that deletes a document, carrying what the
    /// validation read about the contract, the document and its removal record
    pub fn from_borrowed_transition_with_document_deletion(
        value: &ContractUserModerationTransitionV0,
        document_deletion: ContractDocumentDeletionContext,
    ) -> Self {
        let mut action = Self::from_borrowed_transition_with_status(
            value,
            &ContractModerationStatus::default(),
            0,
        );
        action.document_deletion = Some(document_deletion);
        action
    }

    /// The action of a borrowed transition that restores a document, carrying what the
    /// validation read and decoded: the contract, the document and its marked record
    pub fn from_borrowed_transition_with_document_restoration(
        value: &ContractUserModerationTransitionV0,
        document_restoration: ContractDocumentRestorationContext,
    ) -> Self {
        let mut action = Self::from_borrowed_transition_with_status(
            value,
            &ContractModerationStatus::default(),
            0,
        );
        action.document_restoration = Some(document_restoration);
        action
    }
}
