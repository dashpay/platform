use std::fmt;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::token_transition::TokenTransition;
use crate::ProtocolError;

// @append-only
#[derive(Eq, PartialEq, Debug, Copy, Clone, Hash)]
pub enum TokenTransitionActionType {
    Burn,
    Mint,
    Transfer,
    Freeze,
    Unfreeze,
    DestroyFrozenFunds,
    Claim,
    EmergencyAction,
    ConfigUpdate,
    DirectPurchase,
    SetPriceForDirectPurchase,
}

impl fmt::Display for TokenTransitionActionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let action_str = match self {
            TokenTransitionActionType::Burn => "Burn",
            TokenTransitionActionType::Mint => "Mint",
            TokenTransitionActionType::Transfer => "Transfer",
            TokenTransitionActionType::Freeze => "Freeze",
            TokenTransitionActionType::Unfreeze => "Unfreeze",
            TokenTransitionActionType::DestroyFrozenFunds => "DestroyFrozenFunds",
            TokenTransitionActionType::Claim => "Claim",
            TokenTransitionActionType::EmergencyAction => "EmergencyAction",
            TokenTransitionActionType::ConfigUpdate => "ConfigUpdate",
            TokenTransitionActionType::DirectPurchase => "DirectPurchase",
            TokenTransitionActionType::SetPriceForDirectPurchase => "SetPriceForDirectPurchase",
        };
        write!(f, "{}", action_str)
    }
}

pub trait TokenTransitionActionTypeGetter {
    fn action_type(&self) -> TokenTransitionActionType;
}

impl TokenTransitionActionTypeGetter for TokenTransition {
    fn action_type(&self) -> TokenTransitionActionType {
        match self {
            TokenTransition::Burn(_) => TokenTransitionActionType::Burn,
            TokenTransition::Mint(_) => TokenTransitionActionType::Mint,
            TokenTransition::Transfer(_) => TokenTransitionActionType::Transfer,
            TokenTransition::Freeze(_) => TokenTransitionActionType::Freeze,
            TokenTransition::Unfreeze(_) => TokenTransitionActionType::Unfreeze,
            TokenTransition::DestroyFrozenFunds(_) => TokenTransitionActionType::DestroyFrozenFunds,
            TokenTransition::Claim(_) => TokenTransitionActionType::Claim,
            TokenTransition::EmergencyAction(_) => TokenTransitionActionType::EmergencyAction,
            TokenTransition::ConfigUpdate(_) => TokenTransitionActionType::ConfigUpdate,
            TokenTransition::SetPriceForDirectPurchase(_) => {
                TokenTransitionActionType::SetPriceForDirectPurchase
            }
            TokenTransition::DirectPurchase(_) => TokenTransitionActionType::DirectPurchase,
        }
    }
}

impl TryFrom<&str> for TokenTransitionActionType {
    type Error = ProtocolError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "burn" => Ok(TokenTransitionActionType::Burn),
            "issuance" => Ok(TokenTransitionActionType::Mint),
            "transfer" => Ok(TokenTransitionActionType::Transfer),
            "freeze" => Ok(TokenTransitionActionType::Freeze),
            "unfreeze" => Ok(TokenTransitionActionType::Unfreeze),
            "claim" => Ok(TokenTransitionActionType::Claim),
            "destroy_frozen_funds" | "destroyFrozenFunds" => {
                Ok(TokenTransitionActionType::DestroyFrozenFunds)
            }
            "emergency_action" | "emergencyAction" => {
                Ok(TokenTransitionActionType::EmergencyAction)
            }
            "config_update" | "configUpdate" => Ok(TokenTransitionActionType::ConfigUpdate),
            "direct_purchase" | "directPurchase" => Ok(TokenTransitionActionType::DirectPurchase),
            "set_price_for_direct_purchase" | "setPriceForDirectPurchase" => {
                Ok(TokenTransitionActionType::SetPriceForDirectPurchase)
            }
            action_type => Err(ProtocolError::Generic(format!(
                "unknown token transition action type {action_type}"
            ))),
        }
    }
}
