use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use dpp::state_transition::batch_transition::batched_transition::token_transition_action_type::{
    TokenTransitionActionType, TokenTransitionActionTypeGetter,
};

impl TokenTransitionActionTypeGetter for TokenTransitionAction {
    fn action_type(&self) -> TokenTransitionActionType {
        match self {
            TokenTransitionAction::BurnAction(_) => TokenTransitionActionType::Burn,
            TokenTransitionAction::MintAction(_) => TokenTransitionActionType::Mint,
            TokenTransitionAction::TransferAction(_) => TokenTransitionActionType::Transfer,
            TokenTransitionAction::FreezeAction(_) => TokenTransitionActionType::Freeze,
            TokenTransitionAction::UnfreezeAction(_) => TokenTransitionActionType::Unfreeze,
            TokenTransitionAction::ClaimAction(_) => TokenTransitionActionType::Claim,
            TokenTransitionAction::EmergencyActionAction(_) => {
                TokenTransitionActionType::EmergencyAction
            }
            TokenTransitionAction::DestroyFrozenFundsAction(_) => {
                TokenTransitionActionType::DestroyFrozenFunds
            }
            TokenTransitionAction::ConfigUpdateAction(_) => TokenTransitionActionType::ConfigUpdate,
            TokenTransitionAction::DirectPurchaseAction(_) => {
                TokenTransitionActionType::DirectPurchase
            }
            TokenTransitionAction::SetPriceForDirectPurchaseAction(_) => {
                TokenTransitionActionType::SetPriceForDirectPurchase
            }
        }
    }
}
