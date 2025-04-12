use std::fmt;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::token_transition::TokenTransition;
use crate::ProtocolError;

// @append-only
/// Represents the type of action described by a token-related state transition.
///
/// `TokenTransitionActionType` is **not used by the backend system directly**,
/// but is intended to assist **client-side applications** in identifying and
/// classifying the purpose of a token state transition.
///
/// This enum enables clients to more easily display, filter, or handle different
/// token operations, such as minting, transferring, or burning tokens.
#[derive(Eq, PartialEq, Debug, Copy, Clone, Hash)]
pub enum TokenTransitionActionType {
    /// Indicates a burn action, where tokens are permanently removed from circulation.
    Burn,

    /// Indicates a mint action, where new tokens are created and added to supply.
    Mint,

    /// Indicates a transfer of tokens between identities.
    Transfer,

    /// Indicates that tokens are being frozen, preventing their use or transfer.
    Freeze,

    /// Indicates that previously frozen tokens are being unfrozen and made usable again.
    Unfreeze,

    /// Indicates the destruction of tokens that were in a frozen state.
    DestroyFrozenFunds,

    /// Indicates a claim action, typically used to redeem or withdraw tokens (e.g., from rewards).
    Claim,

    /// Indicates an emergency action, usually reserved for critical recovery or administrative intervention.
    EmergencyAction,

    /// Indicates a configuration update affecting token properties or behavior.
    ConfigUpdate,

    /// Indicates that the transition involves a direct purchase of tokens.
    DirectPurchase,

    /// Indicates that the transition sets or updates the price for direct token purchases.
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
