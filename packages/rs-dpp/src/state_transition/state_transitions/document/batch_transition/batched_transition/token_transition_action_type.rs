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

#[cfg(test)]
mod tests {
    use super::*;

    // The happy paths and basic error case are already covered in
    // batch_transition/tests.rs. The tests below add coverage for
    // edge cases NOT exercised there.

    #[test]
    fn try_from_str_returns_protocol_error_with_unknown_action_substring() {
        // Verify the specific error message structure (only is_err() is
        // checked elsewhere) — this exercises the format!() in the catch-all.
        let err = TokenTransitionActionType::try_from("not_a_real_action").unwrap_err();
        match err {
            ProtocolError::Generic(msg) => {
                assert!(msg.contains("unknown token transition action type"));
                assert!(msg.contains("not_a_real_action"));
            }
            other => panic!("expected ProtocolError::Generic, got {:?}", other),
        }
    }

    #[test]
    fn try_from_str_errors_for_empty_string() {
        assert!(TokenTransitionActionType::try_from("").is_err());
    }

    #[test]
    fn try_from_str_does_not_accept_mint_keyword() {
        // Known quirk: "issuance" maps to Mint, but "mint" itself is NOT valid.
        // Locks in this surprising aliasing behavior.
        assert!(TokenTransitionActionType::try_from("mint").is_err());
    }

    #[test]
    fn try_from_str_is_case_sensitive_on_basic_variants() {
        assert!(TokenTransitionActionType::try_from("Burn").is_err());
        assert!(TokenTransitionActionType::try_from("BURN").is_err());
        assert!(TokenTransitionActionType::try_from("Transfer").is_err());
    }

    #[test]
    fn try_from_str_does_not_trim_whitespace() {
        assert!(TokenTransitionActionType::try_from(" burn").is_err());
        assert!(TokenTransitionActionType::try_from("burn ").is_err());
        assert!(TokenTransitionActionType::try_from("\tburn").is_err());
    }
}
