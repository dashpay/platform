mod transformer;

use std::sync::Arc;
use dpp::identifier::Identifier;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};

/// Token issuance transition action v0
#[derive(Debug, Clone)]
pub struct TokenSetPriceForDirectPurchaseTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// What should be the price for a single token
    /// Setting this to None makes it no longer purchasable
    pub price: Option<TokenPricingSchedule>,
    /// The public note
    pub public_note: Option<String>,
}

/// Accessors for `TokenIssuanceTransitionActionV0`
pub trait TokenSetPriceForDirectPurchaseTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes self and returns the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// Returns the price
    fn price(&self) -> Option<&TokenPricingSchedule>;

    /// Sets the amount of tokens to issuance
    fn set_price(&mut self, price: Option<TokenPricingSchedule>);

    /// Returns the token position in the contract
    fn token_position(&self) -> u16 {
        self.base().token_position()
    }

    /// Returns the token ID
    fn token_id(&self) -> Identifier {
        self.base().token_id()
    }

    /// Returns the data contract ID
    fn data_contract_id(&self) -> Identifier {
        self.base().data_contract_id()
    }

    /// Returns a reference to the data contract fetch info
    fn data_contract_fetch_info_ref(&self) -> &Arc<DataContractFetchInfo> {
        self.base().data_contract_fetch_info_ref()
    }

    /// Returns the data contract fetch info
    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo> {
        self.base().data_contract_fetch_info()
    }

    /// Returns the public note (optional)
    fn public_note(&self) -> Option<&String>;

    /// Returns the public note (owned)
    fn public_note_owned(self) -> Option<String>;

    /// Sets the public note
    fn set_public_note(&mut self, public_note: Option<String>);
}

impl TokenSetPriceForDirectPurchaseTransitionActionAccessorsV0
    for TokenSetPriceForDirectPurchaseTransitionActionV0
{
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::contract::DataContractFetchInfo;
    use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionV0;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn make_base() -> TokenBaseTransitionAction {
        let fetch_info = DataContractFetchInfo::dpns_contract_fixture(
            PlatformVersion::latest().protocol_version,
        );
        TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
            token_id: Identifier::new([0xAD; 32]),
            identity_contract_nonce: 12,
            token_contract_position: 0,
            data_contract: Arc::new(fetch_info),
            store_in_group: None,
            perform_action: true,
        })
    }

    fn make_v0(
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
    fn v0_price_none_returns_none() {
        let v0 = make_v0(None, None);
        assert!(v0.price().is_none());
    }

    #[test]
    fn v0_price_some_single_returns_reference() {
        let v0 = make_v0(Some(TokenPricingSchedule::SinglePrice(1_234)), None);
        match v0.price() {
            Some(TokenPricingSchedule::SinglePrice(c)) => assert_eq!(*c, 1_234),
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn v0_price_some_set_prices_returns_reference() {
        let mut map = BTreeMap::new();
        map.insert(1u64, 10u64);
        map.insert(100u64, 5u64);
        let v0 = make_v0(Some(TokenPricingSchedule::SetPrices(map.clone())), None);
        match v0.price() {
            Some(TokenPricingSchedule::SetPrices(got)) => assert_eq!(got, &map),
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn v0_set_price_from_none_to_some_updates() {
        let mut v0 = make_v0(None, None);
        v0.set_price(Some(TokenPricingSchedule::SinglePrice(42)));
        match v0.price() {
            Some(TokenPricingSchedule::SinglePrice(c)) => assert_eq!(*c, 42),
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn v0_set_price_to_none_clears() {
        let mut v0 = make_v0(Some(TokenPricingSchedule::SinglePrice(1)), None);
        v0.set_price(None);
        assert!(v0.price().is_none());
    }

    #[test]
    fn v0_public_note_accessors_work() {
        let v0 = make_v0(None, Some("hello"));
        assert_eq!(v0.public_note(), Some(&"hello".to_string()));
        let owned = v0.public_note_owned();
        assert_eq!(owned, Some("hello".to_string()));
    }

    #[test]
    fn v0_set_public_note_round_trip() {
        let mut v0 = make_v0(None, None);
        v0.set_public_note(Some("x".to_string()));
        assert_eq!(v0.public_note(), Some(&"x".to_string()));
        v0.set_public_note(None);
        assert!(v0.public_note().is_none());
    }

    #[test]
    fn v0_default_accessors_delegate_to_base() {
        let v0 = make_v0(None, None);
        assert_eq!(v0.token_position(), 0);
        assert_eq!(v0.token_id(), Identifier::new([0xAD; 32]));
        let fetch = v0.data_contract_fetch_info();
        assert_eq!(v0.data_contract_id(), fetch.contract.id());
        assert!(Arc::ptr_eq(
            v0.data_contract_fetch_info_ref(),
            &v0.data_contract_fetch_info(),
        ));
    }

    #[test]
    fn v0_base_owned_preserves_token_id() {
        let v0 = make_v0(None, None);
        let ref_id = v0.base().token_id();
        let base = v0.base_owned();
        assert_eq!(ref_id, base.token_id());
    }
}
