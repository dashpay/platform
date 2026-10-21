use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::prelude::TimestampMillis;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_place_ask_transition::TokenPlaceAskTransitionV0;

impl TokenBaseTransitionAccessors for TokenPlaceAskTransitionV0 {
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

pub trait TokenPlaceAskTransitionV0Methods: TokenBaseTransitionAccessors {
    fn amount(&self) -> TokenAmount;

    fn set_amount(&mut self, amount: TokenAmount);

    fn price(&self) -> Credits;

    fn set_price(&mut self, price: Credits);

    fn min_fill(&self) -> TokenAmount;

    fn set_min_fill(&mut self, min_fill: TokenAmount);

    fn expires_at(&self) -> Option<TimestampMillis>;

    fn set_expires_at(&mut self, expires_at: Option<TimestampMillis>);
}

impl TokenPlaceAskTransitionV0Methods for TokenPlaceAskTransitionV0 {
    fn amount(&self) -> TokenAmount {
        self.amount
    }

    fn set_amount(&mut self, amount: TokenAmount) {
        self.amount = amount;
    }

    fn price(&self) -> Credits {
        self.price
    }

    fn set_price(&mut self, price: Credits) {
        self.price = price;
    }

    fn min_fill(&self) -> TokenAmount {
        self.min_fill
    }

    fn set_min_fill(&mut self, min_fill: TokenAmount) {
        self.min_fill = min_fill;
    }

    fn expires_at(&self) -> Option<TimestampMillis> {
        self.expires_at
    }

    fn set_expires_at(&mut self, expires_at: Option<TimestampMillis>) {
        self.expires_at = expires_at;
    }
}
