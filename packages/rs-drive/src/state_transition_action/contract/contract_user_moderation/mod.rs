/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::contract::contract_user_moderation::v0::{
    ContractDocumentDeletionContext, ContractUserModerationTransitionActionV0,
};
use derive_more::From;
use dpp::platform_value::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};
use dpp::state_transition::contract_user_moderation_transition::ContractUserModerationAction;

/// The action of a contract user moderation transition: one edit of a moderated contract's
/// banlist or suspension list, with the target's status as it was read when the transition was
/// validated, or the deletion of one document, with what was read about it.
#[derive(Debug, Clone, From)]
pub enum ContractUserModerationTransitionAction {
    /// v0
    V0(ContractUserModerationTransitionActionV0),
}

impl ContractUserModerationTransitionAction {
    /// The moderator that signed
    pub fn moderator_id(&self) -> Identifier {
        match self {
            ContractUserModerationTransitionAction::V0(action) => action.moderator_id,
        }
    }

    /// The moderated contract
    pub fn data_contract_id(&self) -> Identifier {
        match self {
            ContractUserModerationTransitionAction::V0(action) => action.data_contract_id,
        }
    }

    /// The moderator's nonce for the contract
    pub fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            ContractUserModerationTransitionAction::V0(action) => action.identity_contract_nonce,
        }
    }

    /// What is done, to whom
    pub fn action(&self) -> &ContractUserModerationAction {
        match self {
            ContractUserModerationTransitionAction::V0(action) => &action.action,
        }
    }

    /// Whether the target carried a suspension when the transition was validated
    pub fn target_is_suspended(&self) -> bool {
        match self {
            ContractUserModerationTransitionAction::V0(action) => action.target_is_suspended,
        }
    }

    /// What a document deletion read when the transition was validated, `None` for an action
    /// on an identity
    pub fn document_deletion(&self) -> Option<&ContractDocumentDeletionContext> {
        match self {
            ContractUserModerationTransitionAction::V0(action) => action.document_deletion.as_ref(),
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            ContractUserModerationTransitionAction::V0(action) => action.user_fee_increase,
        }
    }
}
