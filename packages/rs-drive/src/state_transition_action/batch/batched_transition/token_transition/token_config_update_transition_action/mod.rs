use derive_more::From;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;

/// transformer module for token freeze transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenIssuanceTransitionActionV0)

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token config update transition action
#[derive(Debug, Clone, From)]
pub enum TokenConfigUpdateTransitionAction {
    /// v0
    V0(TokenConfigUpdateTransitionActionV0),
}

impl TokenConfigUpdateTransitionActionAccessorsV0 for TokenConfigUpdateTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenConfigUpdateTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenConfigUpdateTransitionAction::V0(v0) => v0.base,
        }
    }

    fn update_token_configuration_item(&self) -> &TokenConfigurationChangeItem {
        match self {
            TokenConfigUpdateTransitionAction::V0(v0) => &v0.update_token_configuration_item,
        }
    }

    fn set_update_token_configuration_item(
        &mut self,
        update_token_configuration_item: TokenConfigurationChangeItem,
    ) {
        match self {
            TokenConfigUpdateTransitionAction::V0(v0) => {
                v0.update_token_configuration_item = update_token_configuration_item
            }
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenConfigUpdateTransitionAction::V0(v0) => v0.public_note.as_ref(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenConfigUpdateTransitionAction::V0(v0) => v0.public_note,
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenConfigUpdateTransitionAction::V0(v0) => v0.public_note = public_note,
        }
    }
}
