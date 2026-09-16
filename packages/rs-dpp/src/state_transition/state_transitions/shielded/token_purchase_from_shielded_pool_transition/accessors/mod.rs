mod v0;
pub use v0::*;

use crate::balances::credits::TokenAmount;
use crate::data_contract::TokenContractPosition;
use crate::fee::Credits;
use crate::shielded::SerializedAction;
use crate::state_transition::token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransition;
use platform_value::Identifier;

impl TokenPurchaseFromShieldedPoolTransitionAccessorsV0
    for TokenPurchaseFromShieldedPoolTransition
{
    fn data_contract_id(&self) -> Identifier {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(v0) => v0.data_contract_id,
        }
    }
    fn token_contract_position(&self) -> TokenContractPosition {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(v0) => v0.token_contract_position,
        }
    }
    fn token_id(&self) -> Identifier {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(v0) => v0.token_id,
        }
    }
    fn token_count(&self) -> TokenAmount {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(v0) => v0.token_count,
        }
    }
    fn total_agreed_price(&self) -> Credits {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(v0) => v0.total_agreed_price,
        }
    }
    fn token_actions(&self) -> &[SerializedAction] {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(v0) => &v0.token_actions,
        }
    }
    fn token_anchor(&self) -> [u8; 32] {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(v0) => v0.token_anchor,
        }
    }
    fn token_proof(&self) -> &[u8] {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(v0) => &v0.token_proof,
        }
    }
    fn token_binding_signature(&self) -> [u8; 64] {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(v0) => v0.token_binding_signature,
        }
    }
    fn fee_actions(&self) -> &[SerializedAction] {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(v0) => &v0.fee_actions,
        }
    }
    fn fee_anchor(&self) -> [u8; 32] {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(v0) => v0.fee_anchor,
        }
    }
    fn fee_proof(&self) -> &[u8] {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(v0) => &v0.fee_proof,
        }
    }
    fn fee_binding_signature(&self) -> [u8; 64] {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(v0) => v0.fee_binding_signature,
        }
    }
    fn credit_amount(&self) -> Credits {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(v0) => v0.credit_amount,
        }
    }
}
