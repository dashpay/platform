mod v0;
pub use v0::*;

use crate::balances::credits::TokenAmount;
use crate::data_contract::TokenContractPosition;
use crate::fee::Credits;
use crate::shielded::SerializedAction;
use crate::state_transition::token_unshield_with_shielded_fee_transition::TokenUnshieldWithShieldedFeeTransition;
use platform_value::Identifier;

impl TokenUnshieldWithShieldedFeeTransitionAccessorsV0 for TokenUnshieldWithShieldedFeeTransition {
    fn data_contract_id(&self) -> Identifier {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(v0) => v0.data_contract_id,
        }
    }
    fn token_contract_position(&self) -> TokenContractPosition {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(v0) => v0.token_contract_position,
        }
    }
    fn token_id(&self) -> Identifier {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(v0) => v0.token_id,
        }
    }
    fn recipient_id(&self) -> Identifier {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(v0) => v0.recipient_id,
        }
    }
    fn amount(&self) -> TokenAmount {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(v0) => v0.amount,
        }
    }
    fn token_actions(&self) -> &[SerializedAction] {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(v0) => &v0.token_actions,
        }
    }
    fn token_anchor(&self) -> [u8; 32] {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(v0) => v0.token_anchor,
        }
    }
    fn token_proof(&self) -> &[u8] {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(v0) => &v0.token_proof,
        }
    }
    fn token_binding_signature(&self) -> [u8; 64] {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(v0) => v0.token_binding_signature,
        }
    }
    fn fee_actions(&self) -> &[SerializedAction] {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(v0) => &v0.fee_actions,
        }
    }
    fn fee_anchor(&self) -> [u8; 32] {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(v0) => v0.fee_anchor,
        }
    }
    fn fee_proof(&self) -> &[u8] {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(v0) => &v0.fee_proof,
        }
    }
    fn fee_binding_signature(&self) -> [u8; 64] {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(v0) => v0.fee_binding_signature,
        }
    }
    fn credit_amount(&self) -> Credits {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(v0) => v0.credit_amount,
        }
    }
}
