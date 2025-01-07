mod token_transition_action_type;

/// token_base_transition_action
pub mod token_base_transition_action;
/// token_burn_transition_action
pub mod token_burn_transition_action;
/// token_freeze_transition_action
pub mod token_freeze_transition_action;
/// token_issuance_transition_action
pub mod token_mint_transition_action;
/// token_transfer_transition_action
pub mod token_transfer_transition_action;
/// token_unfreeze_transition_action
pub mod token_unfreeze_transition_action;

/// token_destroy_frozen_funds_transition_action
pub mod token_destroy_frozen_funds_transition_action;
/// token_emergency_action_transition_action
pub mod token_emergency_action_transition_action;

use derive_more::From;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_burn_transition_action::{TokenBurnTransitionAction, TokenBurnTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_freeze_transition_action::{TokenFreezeTransitionAction, TokenFreezeTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_unfreeze_transition_action::{TokenUnfreezeTransitionAction, TokenUnfreezeTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_mint_transition_action::{TokenMintTransitionAction, TokenMintTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::TokenTransferTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::TokenTransferTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_emergency_action_transition_action::TokenEmergencyActionTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_emergency_action_transition_action::TokenEmergencyActionTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::TokenDestroyFrozenFundsTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::TokenDestroyFrozenFundsTransitionActionAccessorsV0;

/// token action
#[derive(Debug, Clone, From)]
pub enum TokenTransitionAction {
    /// burn
    BurnAction(TokenBurnTransitionAction),
    /// issuance
    MintAction(TokenMintTransitionAction),
    /// transfer
    TransferAction(TokenTransferTransitionAction),
    /// freeze
    FreezeAction(TokenFreezeTransitionAction),
    /// unfreeze
    UnfreezeAction(TokenUnfreezeTransitionAction),
    /// emergency action
    EmergencyActionAction(TokenEmergencyActionTransitionAction),
    /// destroy frozen funds action
    DestroyFrozenFundsAction(TokenDestroyFrozenFundsTransitionAction),
}

impl TokenTransitionAction {
    /// Returns a reference to the base token transition action if available
    pub fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenTransitionAction::BurnAction(action) => action.base(),
            TokenTransitionAction::MintAction(action) => action.base(),
            TokenTransitionAction::TransferAction(action) => action.base(),
            TokenTransitionAction::FreezeAction(action) => action.base(),
            TokenTransitionAction::UnfreezeAction(action) => action.base(),
            TokenTransitionAction::EmergencyActionAction(action) => action.base(),
            TokenTransitionAction::DestroyFrozenFundsAction(action) => action.base(),
        }
    }

    /// Consumes self and returns the base token transition action if available
    pub fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenTransitionAction::BurnAction(action) => action.base_owned(),
            TokenTransitionAction::MintAction(action) => action.base_owned(),
            TokenTransitionAction::TransferAction(action) => action.base_owned(),
            TokenTransitionAction::FreezeAction(action) => action.base_owned(),
            TokenTransitionAction::UnfreezeAction(action) => action.base_owned(),
            TokenTransitionAction::EmergencyActionAction(action) => action.base_owned(),
            TokenTransitionAction::DestroyFrozenFundsAction(action) => action.base_owned(),
        }
    }
}
