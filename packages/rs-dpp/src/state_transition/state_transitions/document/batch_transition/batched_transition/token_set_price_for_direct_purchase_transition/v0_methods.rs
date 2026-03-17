use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use crate::prelude::IdentityNonce;
use crate::ProtocolError;
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
    fn calculate_action_id(
        &self,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Identifier, ProtocolError> {
        match self {
            TokenSetPriceForDirectPurchaseTransition::V0(v0) => {
                v0.calculate_action_id(owner_id, platform_version)
            }
        }
    }
}

impl TokenSetPriceForDirectPurchaseTransition {
    /// Computes the action_id by hashing the full serialized pricing schedule.
    ///
    /// Previous versions only hashed the minimum-tier credit price, which meant
    /// different pricing schedules with the same minimum price (e.g.,
    /// `SetPrices({1: 100, 10: 800})` vs `SetPrices({1: 100, 10: 9999})`)
    /// produced identical action_ids, enabling vote-swap attacks.
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
            let serialized =
                bincode::encode_to_vec(price_per_token, bincode::config::standard())
                    .expect("expected to encode pricing schedule");
            bytes.extend_from_slice(&serialized);
        }

        hash_double(bytes).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use crate::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::TokenSetPriceForDirectPurchaseTransitionV0;
    use std::collections::BTreeMap;

    fn make_transition(
        price: Option<TokenPricingSchedule>,
    ) -> TokenSetPriceForDirectPurchaseTransition {
        TokenSetPriceForDirectPurchaseTransition::V0(
            TokenSetPriceForDirectPurchaseTransitionV0 {
                base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                    identity_contract_nonce: 1,
                    token_contract_position: 0,
                    data_contract_id: Identifier::new([1u8; 32]),
                    token_id: Identifier::new([2u8; 32]),
                    using_group_info: None,
                }),
                price,
                public_note: None,
            },
        )
    }

    #[test]
    fn different_set_prices_with_same_minimum_produce_different_ids() {
        // This was the vulnerability: two SetPrices schedules with the same
        // minimum-tier price but different higher tiers produced identical
        // action_ids when only minimum_purchase_amount_and_price().1 was hashed.
        let owner_id = Identifier::new([3u8; 32]);

        let t_cheap = make_transition(Some(TokenPricingSchedule::SetPrices(
            BTreeMap::from([(1, 100), (10, 800)]),
        )));
        let t_expensive = make_transition(Some(TokenPricingSchedule::SetPrices(
            BTreeMap::from([(1, 100), (10, 9999)]),
        )));

        let id_cheap = t_cheap.calculate_action_id(owner_id);
        let id_expensive = t_expensive.calculate_action_id(owner_id);

        assert_ne!(
            id_cheap, id_expensive,
            "different pricing schedules with same minimum price must produce different action_ids"
        );
    }

    #[test]
    fn single_price_and_set_prices_with_same_minimum_produce_different_ids() {
        // SinglePrice(100) and SetPrices({1: 100}) both have
        // minimum_purchase_amount_and_price() == (1, 100), but they are
        // semantically different schedules.
        let owner_id = Identifier::new([3u8; 32]);

        let t_single = make_transition(Some(TokenPricingSchedule::SinglePrice(100)));
        let t_set = make_transition(Some(TokenPricingSchedule::SetPrices(
            BTreeMap::from([(1, 100)]),
        )));

        let id_single = t_single.calculate_action_id(owner_id);
        let id_set = t_set.calculate_action_id(owner_id);

        assert_ne!(
            id_single, id_set,
            "SinglePrice and SetPrices with same minimum must produce different action_ids"
        );
    }

    #[test]
    fn identical_schedules_produce_same_id() {
        let owner_id = Identifier::new([3u8; 32]);

        let t1 = make_transition(Some(TokenPricingSchedule::SetPrices(
            BTreeMap::from([(1, 100), (10, 800)]),
        )));
        let t2 = make_transition(Some(TokenPricingSchedule::SetPrices(
            BTreeMap::from([(1, 100), (10, 800)]),
        )));

        assert_eq!(
            t1.calculate_action_id(owner_id),
            t2.calculate_action_id(owner_id),
            "identical pricing schedules must produce the same action_id"
        );
    }

    #[test]
    fn none_price_produces_different_id_from_some_price() {
        let owner_id = Identifier::new([3u8; 32]);

        let t_none = make_transition(None);
        let t_some = make_transition(Some(TokenPricingSchedule::SinglePrice(100)));

        assert_ne!(
            t_none.calculate_action_id(owner_id),
            t_some.calculate_action_id(owner_id),
            "None price and Some price must produce different action_ids"
        );
    }
}
