use crate::state_transition_action::contract::contract_user_moderation::v0::{
    ContractDocumentDeletionContext, ContractDocumentRestorationContext,
    ContractUserModerationTransitionActionV0,
};
use crate::state_transition_action::contract::contract_user_moderation::ContractUserModerationTransitionAction;
use dpp::data_contract::config::moderation::ContractModerationStatus;
use dpp::identity::TimestampMillis;
use dpp::state_transition::contract_user_moderation_transition::ContractUserModerationTransition;

impl ContractUserModerationTransitionAction {
    /// The action of a borrowed transition, keeping of the target's stored status what Drive
    /// needs: whether it carries a suspension, and for a warn the warnings it carries and
    /// `block_time_ms`, the time the new warning is stamped with
    pub fn from_borrowed_transition_with_status(
        value: &ContractUserModerationTransition,
        current_status: &ContractModerationStatus,
        block_time_ms: TimestampMillis,
    ) -> Self {
        match value {
            ContractUserModerationTransition::V0(v0) => {
                ContractUserModerationTransitionActionV0::from_borrowed_transition_with_status(
                    v0,
                    current_status,
                    block_time_ms,
                )
                .into()
            }
        }
    }

    /// The action of a borrowed transition that deletes a document, carrying what the
    /// validation read about the contract, the document and its removal record
    pub fn from_borrowed_transition_with_document_deletion(
        value: &ContractUserModerationTransition,
        document_deletion: ContractDocumentDeletionContext,
    ) -> Self {
        match value {
            ContractUserModerationTransition::V0(v0) => {
                ContractUserModerationTransitionActionV0::from_borrowed_transition_with_document_deletion(
                    v0,
                    document_deletion,
                )
                .into()
            }
        }
    }

    /// The action of a borrowed transition that restores a document, carrying what the
    /// validation read and decoded: the contract, the document and its marked record
    pub fn from_borrowed_transition_with_document_restoration(
        value: &ContractUserModerationTransition,
        document_restoration: ContractDocumentRestorationContext,
    ) -> Self {
        match value {
            ContractUserModerationTransition::V0(v0) => {
                ContractUserModerationTransitionActionV0::from_borrowed_transition_with_document_restoration(
                    v0,
                    document_restoration,
                )
                .into()
            }
        }
    }
}
