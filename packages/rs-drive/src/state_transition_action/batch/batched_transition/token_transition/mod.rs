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

/// token_config_update_transition_action
pub mod token_config_update_transition_action;
/// token_destroy_frozen_funds_transition_action
pub mod token_destroy_frozen_funds_transition_action;
/// token_emergency_action_transition_action
pub mod token_emergency_action_transition_action;

/// token_claim_transition_action
pub mod token_claim_transition_action;
/// token_order_adjust_price_transition_action
pub mod token_order_adjust_price_transition_action;
/// token_order_buy_limit_transition_action
pub mod token_order_buy_limit_transition_action;
/// token_order_cancel_transition_action
pub mod token_order_cancel_transition_action;
/// token_order_sell_limit_transition_action
pub mod token_order_sell_limit_transition_action;

use derive_more::From;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::TokenKeepsHistoryRulesV0Getters;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_burn_transition_action::{TokenBurnTransitionAction, TokenBurnTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_config_update_transition_action::{TokenConfigUpdateTransitionAction, TokenConfigUpdateTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_freeze_transition_action::{TokenFreezeTransitionAction, TokenFreezeTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_unfreeze_transition_action::{TokenUnfreezeTransitionAction, TokenUnfreezeTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_mint_transition_action::{TokenMintTransitionAction, TokenMintTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::TokenTransferTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::TokenTransferTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_emergency_action_transition_action::TokenEmergencyActionTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_emergency_action_transition_action::TokenEmergencyActionTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::TokenDestroyFrozenFundsTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::TokenDestroyFrozenFundsTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_claim_transition_action::{TokenClaimTransitionAction, TokenClaimTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_order_adjust_price_transition_action::action::TokenOrderAdjustPriceTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_order_adjust_price_transition_action::TokenOrderAdjustPriceTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_order_buy_limit_transition_action::action::TokenOrderBuyLimitTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_order_buy_limit_transition_action::TokenOrderBuyLimitTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_order_cancel_transition_action::action::TokenOrderCancelTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_order_cancel_transition_action::TokenOrderCancelTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_order_sell_limit_transition_action::action::TokenOrderSellLimitTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_order_sell_limit_transition_action::TokenOrderSellLimitTransitionActionAccessorsV0;

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
    /// release
    ClaimAction(TokenClaimTransitionAction),
    /// emergency action
    EmergencyActionAction(TokenEmergencyActionTransitionAction),
    /// destroy frozen funds action
    DestroyFrozenFundsAction(TokenDestroyFrozenFundsTransitionAction),
    /// update the token configuration
    ConfigUpdateAction(TokenConfigUpdateTransitionAction),
    /// order buy limit
    OrderBuyLimitAction(TokenOrderBuyLimitTransitionAction),
    /// order sell limit
    OrderSellLimitAction(TokenOrderSellLimitTransitionAction),
    /// order cancel
    OrderCancelAction(TokenOrderCancelTransitionAction),
    /// order adjust price
    OrderAdjustPriceAction(TokenOrderAdjustPriceTransitionAction),
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
            TokenTransitionAction::ClaimAction(action) => action.base(),
            TokenTransitionAction::EmergencyActionAction(action) => action.base(),
            TokenTransitionAction::DestroyFrozenFundsAction(action) => action.base(),
            TokenTransitionAction::ConfigUpdateAction(action) => action.base(),
            TokenTransitionAction::OrderBuyLimitAction(action) => action.base(),
            TokenTransitionAction::OrderSellLimitAction(action) => action.base(),
            TokenTransitionAction::OrderCancelAction(action) => action.base(),
            TokenTransitionAction::OrderAdjustPriceAction(action) => action.base(),
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
            TokenTransitionAction::ClaimAction(action) => action.base_owned(),
            TokenTransitionAction::EmergencyActionAction(action) => action.base_owned(),
            TokenTransitionAction::DestroyFrozenFundsAction(action) => action.base_owned(),
            TokenTransitionAction::ConfigUpdateAction(action) => action.base_owned(),
            TokenTransitionAction::OrderBuyLimitAction(action) => action.base_owned(),
            TokenTransitionAction::OrderSellLimitAction(action) => action.base_owned(),
            TokenTransitionAction::OrderCancelAction(action) => action.base_owned(),
            TokenTransitionAction::OrderAdjustPriceAction(action) => action.base_owned(),
        }
    }

    /// Do we keep history for this action
    pub fn keeps_history(&self) -> Result<bool, Error> {
        let keeps_history = self.base().token_configuration()?.keeps_history();
        match self {
            TokenTransitionAction::BurnAction(_) => Ok(keeps_history.keeps_burning_history()),
            TokenTransitionAction::MintAction(_) => Ok(keeps_history.keeps_minting_history()),
            TokenTransitionAction::TransferAction(_) => Ok(keeps_history.keeps_transfer_history()),
            TokenTransitionAction::FreezeAction(_) => Ok(keeps_history.keeps_freezing_history()),
            TokenTransitionAction::UnfreezeAction(_) => Ok(keeps_history.keeps_freezing_history()),
            TokenTransitionAction::ClaimAction(_) => Ok(true),
            TokenTransitionAction::EmergencyActionAction(_) => Ok(true),
            TokenTransitionAction::DestroyFrozenFundsAction(_) => Ok(true),
            TokenTransitionAction::ConfigUpdateAction(_) => Ok(true),
            // TODO: Back to it
            TokenTransitionAction::OrderBuyLimitAction(_) => Ok(false),
            TokenTransitionAction::OrderSellLimitAction(_) => Ok(false),
            TokenTransitionAction::OrderCancelAction(_) => Ok(false),
            TokenTransitionAction::OrderAdjustPriceAction(_) => Ok(false),
        }
    }
}
