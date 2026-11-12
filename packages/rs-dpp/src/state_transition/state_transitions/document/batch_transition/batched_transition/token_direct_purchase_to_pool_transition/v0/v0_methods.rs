use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::shielded::SerializedAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_direct_purchase_to_pool_transition::TokenDirectPurchaseToPoolTransitionV0;

impl TokenBaseTransitionAccessors for TokenDirectPurchaseToPoolTransitionV0 {
    fn base(&self) -> &TokenBaseTransition {
        &self.base
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        &mut self.base
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        self.base = base;
    }
}

pub trait TokenDirectPurchaseToPoolTransitionV0Methods: TokenBaseTransitionAccessors {
    /// Tokens purchased and minted into the pool.
    fn token_count(&self) -> TokenAmount;
    /// The most credits the buyer agrees to pay for `token_count`.
    fn total_agreed_price(&self) -> Credits;
    /// Orchard actions.
    fn actions(&self) -> &[SerializedAction];
    /// Sinsemilla root of the token pool's note commitment tree (Orchard anchor).
    fn anchor(&self) -> &[u8; 32];
    /// Halo 2 proof bytes.
    fn proof(&self) -> &[u8];
    /// RedPallas binding signature.
    fn binding_signature(&self) -> &[u8; 64];
    /// Sets `token_count`.
    fn set_token_count(&mut self, token_count: TokenAmount);
    /// Sets `total_agreed_price`.
    fn set_total_agreed_price(&mut self, total_agreed_price: Credits);
}

impl TokenDirectPurchaseToPoolTransitionV0Methods for TokenDirectPurchaseToPoolTransitionV0 {
    fn token_count(&self) -> TokenAmount {
        self.token_count
    }
    fn total_agreed_price(&self) -> Credits {
        self.total_agreed_price
    }
    fn actions(&self) -> &[SerializedAction] {
        &self.actions
    }
    fn anchor(&self) -> &[u8; 32] {
        &self.anchor
    }
    fn proof(&self) -> &[u8] {
        &self.proof
    }
    fn binding_signature(&self) -> &[u8; 64] {
        &self.binding_signature
    }
    fn set_token_count(&mut self, token_count: TokenAmount) {
        self.token_count = token_count;
    }
    fn set_total_agreed_price(&mut self, total_agreed_price: Credits) {
        self.total_agreed_price = total_agreed_price;
    }
}
