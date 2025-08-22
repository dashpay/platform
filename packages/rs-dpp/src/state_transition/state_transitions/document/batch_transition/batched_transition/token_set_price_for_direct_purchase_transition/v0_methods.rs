use platform_value::Identifier;
use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::TokenSetPriceForDirectPurchaseTransition;
use crate::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::v0::v0_methods::TokenSetPriceForDirectPurchaseTransitionV0Methods;
use crate::tokens::token_pricing_schedule::TokenPricingSchedule;
use crate::util::hash::hash_double;

impl TokenBaseTransitionAccessors for TokenSetPriceForDirectPurchaseTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenSetPriceForDirectPurchaseTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenSetPriceForDirectPurchaseTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenSetPriceForDirectPurchaseTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenSetPriceForDirectPurchaseTransitionV0Methods
    for TokenSetPriceForDirectPurchaseTransition
{
    fn price(&self) -> Option<&TokenPricingSchedule> {
        match self {
            TokenSetPriceForDirectPurchaseTransition::V0(v0) => v0.price(),
        }
    }

    fn set_price(&mut self, price: Option<TokenPricingSchedule>) {
        match self {
            TokenSetPriceForDirectPurchaseTransition::V0(v0) => v0.set_price(price),
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenSetPriceForDirectPurchaseTransition::V0(v0) => v0.public_note(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenSetPriceForDirectPurchaseTransition::V0(v0) => v0.public_note_owned(),
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenSetPriceForDirectPurchaseTransition::V0(v0) => v0.set_public_note(public_note),
        }
    }
}

impl AllowedAsMultiPartyAction for TokenSetPriceForDirectPurchaseTransition {
    fn calculate_action_id(&self, owner_id: Identifier) -> Identifier {
        match self {
            TokenSetPriceForDirectPurchaseTransition::V0(v0) => v0.calculate_action_id(owner_id),
        }
    }
}

impl TokenSetPriceForDirectPurchaseTransition {
    pub fn calculate_action_id_with_fields(
        token_id: &[u8; 32],
        owner_id: &[u8; 32],
        identity_contract_nonce: IdentityNonce,
        price_per_token: Option<&TokenPricingSchedule>,
    ) -> Identifier {
        let mut bytes = b"action_token_set_price_for_direct_purchase".to_vec();
        bytes.extend_from_slice(token_id);
        bytes.extend_from_slice(owner_id);
        bytes.extend_from_slice(&identity_contract_nonce.to_be_bytes());
        if let Some(price_per_token) = price_per_token {
            bytes.extend_from_slice(
                &price_per_token
                    .minimum_purchase_amount_and_price()
                    .1
                    .to_be_bytes(),
            );
        }

        hash_double(bytes).into()
    }
}
