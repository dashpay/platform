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
}

impl fmt::Display for TokenTransitionActionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let action_str = match self {
            TokenTransitionActionType::Burn => "Burn",
            TokenTransitionActionType::Mint => "Mint",
            TokenTransitionActionType::Transfer => "Transfer",
            TokenTransitionActionType::Freeze => "Freeze",
            TokenTransitionActionType::Unfreeze => "Unfreeze",
        };
        write!(f, "{}", action_str)
    }
}

pub trait TransitionActionTypeGetter {
    fn action_type(&self) -> TokenTransitionActionType;
}

impl TransitionActionTypeGetter for TokenTransition {
    fn action_type(&self) -> TokenTransitionActionType {
        match self {
            TokenTransition::Burn(_) => TokenTransitionActionType::Burn,
            TokenTransition::Mint(_) => TokenTransitionActionType::Mint,
            TokenTransition::Transfer(_) => TokenTransitionActionType::Transfer,
            TokenTransition::Freeze(_) => TokenTransitionActionType::Freeze,
            TokenTransition::Unfreeze(_) => TokenTransitionActionType::Unfreeze,
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
            action_type => Err(ProtocolError::Generic(format!(
                "unknown token transition action type {action_type}"
            ))),
        }
    }
}
