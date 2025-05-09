use platform_value::Identifier;
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::TokenSetPriceForDirectPurchaseTransitionV0;
use crate::state_transition::batch_transition::TokenSetPriceForDirectPurchaseTransition;
use crate::tokens::token_pricing_schedule::TokenPricingSchedule;

impl TokenBaseTransitionAccessors for TokenSetPriceForDirectPurchaseTransitionV0 {
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

pub trait TokenSetPriceForDirectPurchaseTransitionV0Methods:
    TokenBaseTransitionAccessors + AllowedAsMultiPartyAction
{
    fn price(&self) -> Option<&TokenPricingSchedule>;

    fn set_price(&mut self, price: Option<TokenPricingSchedule>);

    /// Returns the `public_note` field of the `TokenSetPriceForDirectPurchaseTransitionV0`.
    fn public_note(&self) -> Option<&String>;

    /// Returns the owned `public_note` field of the `TokenSetPriceForDirectPurchaseTransitionV0`.
    fn public_note_owned(self) -> Option<String>;

    /// Sets the value of the `public_note` field in the `TokenSetPriceForDirectPurchaseTransitionV0`.
    fn set_public_note(&mut self, public_note: Option<String>);
}

impl TokenSetPriceForDirectPurchaseTransitionV0Methods
    for TokenSetPriceForDirectPurchaseTransitionV0
{
    fn price(&self) -> Option<&TokenPricingSchedule> {
        self.price.as_ref()
    }

    fn set_price(&mut self, price: Option<TokenPricingSchedule>) {
        self.price = price;
    }

    fn public_note(&self) -> Option<&String> {
        self.public_note.as_ref()
    }

    fn public_note_owned(self) -> Option<String> {
        self.public_note
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        self.public_note = public_note;
    }
}

impl AllowedAsMultiPartyAction for TokenSetPriceForDirectPurchaseTransitionV0 {
    fn calculate_action_id(&self, owner_id: Identifier) -> Identifier {
        let TokenSetPriceForDirectPurchaseTransitionV0 { base, price, .. } = self;

        TokenSetPriceForDirectPurchaseTransition::calculate_action_id_with_fields(
            base.token_id().as_bytes(),
            owner_id.as_bytes(),
            base.identity_contract_nonce(),
            price.as_ref(),
        )
    }
}
