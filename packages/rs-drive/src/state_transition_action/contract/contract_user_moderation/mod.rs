/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::contract::contract_user_moderation::v0::{
    ContractDocumentDeletionContext, ContractUserModerationTransitionActionV0,
    ContractWarningContext,
};
use derive_more::From;
use dpp::platform_value::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};
use dpp::state_transition::contract_user_moderation_transition::ContractUserModerationAction;

/// The action of a contract user moderation transition: one edit of a moderated contract's
/// banlist, suspension list or warning list, with the target's status as it was read when the
/// transition was validated, or the deletion of one document, with what was read about it.
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

    /// What a warn read when the transition was validated, `None` for every other action
    pub fn warning(&self) -> Option<&ContractWarningContext> {
        match self {
            ContractUserModerationTransitionAction::V0(action) => action.warning.as_ref(),
        }
    }

    /// What a document deletion read when the transition was validated, `None` for an action
    /// on an identity
    pub fn document_deletion(&self) -> Option<&ContractDocumentDeletionContext> {
        match self {
            ContractUserModerationTransitionAction::V0(action) => action.document_deletion.as_ref(),
        }
    }

    /// The signer's moderation action count on the elected contract after this action, when
    /// the action counts for a member of the seated team
    pub fn moderation_action_count(&self) -> Option<u32> {
        match self {
            ContractUserModerationTransitionAction::V0(action) => action.moderation_action_count,
        }
    }

    /// The same action, counted for its signer, a member of the elected contract's seated team:
    /// `count` is the signer's moderation action count since the moderators pot was last
    /// settled, this action included
    pub fn with_moderation_action_count(self, count: u32) -> Self {
        match self {
            ContractUserModerationTransitionAction::V0(mut action) => {
                action.moderation_action_count = Some(count);
                ContractUserModerationTransitionAction::V0(action)
            }
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            ContractUserModerationTransitionAction::V0(action) => action.user_fee_increase,
        }
    }
}
