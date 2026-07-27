use derive_more::From;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;

/// transformer module for token issuance transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenIssuanceTransitionActionV0)

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token issuance transition action
#[derive(Debug, Clone, From)]
pub enum TokenSetPriceForDirectPurchaseTransitionAction {
    /// v0
    V0(TokenSetPriceForDirectPurchaseTransitionActionV0),
}

impl TokenSetPriceForDirectPurchaseTransitionActionAccessorsV0
    for TokenSetPriceForDirectPurchaseTransitionAction
{
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenSetPriceForDirectPurchaseTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenSetPriceForDirectPurchaseTransitionAction::V0(v0) => v0.base,
        }
    }

    fn price(&self) -> Option<&TokenPricingSchedule> {
        match self {
            TokenSetPriceForDirectPurchaseTransitionAction::V0(v0) => v0.price.as_ref(),
        }
    }

    fn set_price(&mut self, price: Option<TokenPricingSchedule>) {
        match self {
            TokenSetPriceForDirectPurchaseTransitionAction::V0(v0) => v0.price = price,
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenSetPriceForDirectPurchaseTransitionAction::V0(v0) => v0.public_note.as_ref(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenSetPriceForDirectPurchaseTransitionAction::V0(v0) => v0.public_note,
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenSetPriceForDirectPurchaseTransitionAction::V0(v0) => v0.public_note = public_note,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::contract::DataContractFetchInfo;
    use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{
        TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0,
        TokenBaseTransitionActionV0,
    };
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::identifier::Identifier;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn make_base() -> TokenBaseTransitionAction {
        let fetch_info = DataContractFetchInfo::dpns_contract_fixture(
            PlatformVersion::latest().protocol_version,
        );
        TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
            token_id: Identifier::new([0x70; 32]),
            identity_contract_nonce: 9,
            token_contract_position: 0,
            data_contract: Arc::new(fetch_info),
            store_in_group: None,
            perform_action: true,
        })
    }

    fn make_v0_with(
        price: Option<TokenPricingSchedule>,
        note: Option<&str>,
    ) -> TokenSetPriceForDirectPurchaseTransitionActionV0 {
        TokenSetPriceForDirectPurchaseTransitionActionV0 {
            base: make_base(),
            price,
            public_note: note.map(|s| s.to_string()),
        }
    }

    #[test]
    fn from_v0_wraps_in_enum() {
        let v0 = make_v0_with(Some(TokenPricingSchedule::SinglePrice(42)), Some("n"));
        let wrapped: TokenSetPriceForDirectPurchaseTransitionAction = v0.into();
        assert!(matches!(
            wrapped,
            TokenSetPriceForDirectPurchaseTransitionAction::V0(_)
        ));
    }

    #[test]
    fn enum_base_returns_reference_to_v0_base() {
        let action = TokenSetPriceForDirectPurchaseTransitionAction::V0(make_v0_with(None, None));
        let base = action.base();
        assert_eq!(base.token_id(), Identifier::new([0x70; 32]));
        assert_eq!(base.identity_contract_nonce(), 9);
        assert_eq!(base.token_position(), 0);
    }

    #[test]
    fn enum_base_owned_consumes_self_and_returns_base() {
        let action = TokenSetPriceForDirectPurchaseTransitionAction::V0(make_v0_with(
            Some(TokenPricingSchedule::SinglePrice(100)),
            Some("consume"),
        ));
        let base = action.base_owned();
        assert_eq!(base.token_id(), Identifier::new([0x70; 32]));
    }

    #[test]
    fn enum_price_returns_none_when_unset() {
        let action = TokenSetPriceForDirectPurchaseTransitionAction::V0(make_v0_with(None, None));
        assert!(action.price().is_none());
    }

    #[test]
    fn enum_price_returns_reference_to_single_price() {
        let action = TokenSetPriceForDirectPurchaseTransitionAction::V0(make_v0_with(
            Some(TokenPricingSchedule::SinglePrice(555)),
            None,
        ));
        match action.price() {
            Some(TokenPricingSchedule::SinglePrice(c)) => assert_eq!(*c, 555),
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn enum_price_returns_reference_to_set_prices() {
        let mut map = BTreeMap::new();
        map.insert(1u64, 100u64);
        map.insert(10u64, 80u64);
        let action = TokenSetPriceForDirectPurchaseTransitionAction::V0(make_v0_with(
            Some(TokenPricingSchedule::SetPrices(map.clone())),
            None,
        ));
        match action.price() {
            Some(TokenPricingSchedule::SetPrices(got)) => assert_eq!(got, &map),
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn enum_set_price_from_none_to_some_updates_field() {
        let mut action =
            TokenSetPriceForDirectPurchaseTransitionAction::V0(make_v0_with(None, None));
        assert!(action.price().is_none());

        action.set_price(Some(TokenPricingSchedule::SinglePrice(10_000)));
        match action.price() {
            Some(TokenPricingSchedule::SinglePrice(c)) => assert_eq!(*c, 10_000),
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn enum_set_price_none_clears_existing_price() {
        let mut action = TokenSetPriceForDirectPurchaseTransitionAction::V0(make_v0_with(
            Some(TokenPricingSchedule::SinglePrice(5)),
            None,
        ));
        action.set_price(None);
        assert!(action.price().is_none());
    }

    #[test]
    fn enum_public_note_returns_reference() {
        let action =
            TokenSetPriceForDirectPurchaseTransitionAction::V0(make_v0_with(None, Some("n!")));
        assert_eq!(action.public_note(), Some(&"n!".to_string()));
    }

    #[test]
    fn enum_public_note_owned_consumes_self() {
        let action =
            TokenSetPriceForDirectPurchaseTransitionAction::V0(make_v0_with(None, Some("owned")));
        assert_eq!(action.public_note_owned(), Some("owned".to_string()));
    }

    #[test]
    fn enum_public_note_owned_returns_none_when_unset() {
        let action = TokenSetPriceForDirectPurchaseTransitionAction::V0(make_v0_with(None, None));
        assert!(action.public_note_owned().is_none());
    }

    #[test]
    fn enum_set_public_note_replaces_value() {
        let mut action =
            TokenSetPriceForDirectPurchaseTransitionAction::V0(make_v0_with(None, Some("old")));
        action.set_public_note(Some("newer".to_string()));
        assert_eq!(action.public_note(), Some(&"newer".to_string()));
        action.set_public_note(None);
        assert!(action.public_note().is_none());
    }

    #[test]
    fn v0_accessors_roundtrip_through_price_and_note() {
        let mut v0 = make_v0_with(Some(TokenPricingSchedule::SinglePrice(1)), Some("x"));
        assert_eq!(v0.price(), Some(&TokenPricingSchedule::SinglePrice(1)));
        assert_eq!(v0.public_note(), Some(&"x".to_string()));

        let mut map = BTreeMap::new();
        map.insert(1u64, 99u64);
        v0.set_price(Some(TokenPricingSchedule::SetPrices(map.clone())));
        match v0.price() {
            Some(TokenPricingSchedule::SetPrices(got)) => assert_eq!(got, &map),
            other => panic!("unexpected: {:?}", other),
        }

        v0.set_public_note(None);
        assert!(v0.public_note().is_none());

        let owned_note = v0.clone().public_note_owned();
        assert!(owned_note.is_none());
    }

    #[test]
    fn v0_default_accessors_delegate_to_base() {
        let v0 = make_v0_with(None, None);
        assert_eq!(v0.token_position(), 0);
        assert_eq!(v0.token_id(), Identifier::new([0x70; 32]));
        let fetch = v0.data_contract_fetch_info();
        assert_eq!(v0.data_contract_id(), fetch.contract.id());
        assert!(Arc::ptr_eq(
            v0.data_contract_fetch_info_ref(),
            &v0.data_contract_fetch_info(),
        ));
    }

    #[test]
    fn enum_set_price_overwrites_single_price_with_set_prices() {
        let mut action = TokenSetPriceForDirectPurchaseTransitionAction::V0(make_v0_with(
            Some(TokenPricingSchedule::SinglePrice(1)),
            None,
        ));
        let mut map = BTreeMap::new();
        map.insert(5u64, 500u64);
        action.set_price(Some(TokenPricingSchedule::SetPrices(map)));
        assert!(matches!(
            action.price(),
            Some(TokenPricingSchedule::SetPrices(_))
        ));
    }
}
