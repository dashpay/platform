mod transformer;

use std::sync::Arc;
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};

/// Token issuance transition action v0
#[derive(Debug, Clone)]
pub struct TokenDirectPurchaseTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// How many tokens should we buy.
    pub token_count: TokenAmount,
    /// Agreed price
    /// The user will pay this amount
    pub total_agreed_price: Credits,
}

/// Accessors for `TokenIssuanceTransitionActionV0`
pub trait TokenDirectPurchaseTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes self and returns the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// Returns the amount of tokens to purchase
    fn token_count(&self) -> TokenAmount;

    /// Sets the amount of tokens to purchase
    fn set_token_count(&mut self, amount: TokenAmount);

    /// The agreed price
    fn total_agreed_price(&self) -> Credits;

    /// Sets the agreed price
    fn set_total_agreed_price(&mut self, agreed_price: Credits);

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
}

impl TokenDirectPurchaseTransitionActionAccessorsV0 for TokenDirectPurchaseTransitionActionV0 {
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn token_count(&self) -> TokenAmount {
        self.token_count
    }

    fn set_token_count(&mut self, amount: TokenAmount) {
        self.token_count = amount;
    }

    fn total_agreed_price(&self) -> Credits {
        self.total_agreed_price
    }

    fn set_total_agreed_price(&mut self, agreed_price: Credits) {
        self.total_agreed_price = agreed_price;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::contract::DataContractFetchInfo;
    use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionV0;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::version::PlatformVersion;

    fn make_base() -> TokenBaseTransitionAction {
        let fetch_info = DataContractFetchInfo::dpns_contract_fixture(
            PlatformVersion::latest().protocol_version,
        );
        TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
            token_id: Identifier::new([0xDE; 32]),
            identity_contract_nonce: 2,
            token_contract_position: 0,
            data_contract: Arc::new(fetch_info),
            store_in_group: None,
            perform_action: true,
        })
    }

    fn make_v0(count: TokenAmount, price: Credits) -> TokenDirectPurchaseTransitionActionV0 {
        TokenDirectPurchaseTransitionActionV0 {
            base: make_base(),
            token_count: count,
            total_agreed_price: price,
        }
    }

    #[test]
    fn v0_token_count_returns_stored_value() {
        let v0 = make_v0(123, 456);
        assert_eq!(v0.token_count(), 123);
    }

    #[test]
    fn v0_total_agreed_price_returns_stored_value() {
        let v0 = make_v0(123, 456);
        assert_eq!(v0.total_agreed_price(), 456);
    }

    #[test]
    fn v0_set_token_count_updates() {
        let mut v0 = make_v0(1, 1);
        v0.set_token_count(999);
        assert_eq!(v0.token_count(), 999);
        v0.set_token_count(0);
        assert_eq!(v0.token_count(), 0);
    }

    #[test]
    fn v0_set_total_agreed_price_updates() {
        let mut v0 = make_v0(1, 1);
        v0.set_total_agreed_price(u64::MAX);
        assert_eq!(v0.total_agreed_price(), u64::MAX);
        v0.set_total_agreed_price(0);
        assert_eq!(v0.total_agreed_price(), 0);
    }

    #[test]
    fn v0_default_accessors_delegate_to_base() {
        let v0 = make_v0(1, 1);
        assert_eq!(v0.token_position(), 0);
        assert_eq!(v0.token_id(), Identifier::new([0xDE; 32]));
        let fetch = v0.data_contract_fetch_info();
        assert_eq!(v0.data_contract_id(), fetch.contract.id());
        assert!(Arc::ptr_eq(
            v0.data_contract_fetch_info_ref(),
            &v0.data_contract_fetch_info(),
        ));
    }

    #[test]
    fn v0_base_ref_and_base_owned_preserve_token_id() {
        let v0 = make_v0(1, 1);
        let ref_id = v0.base().token_id();
        let base = v0.base_owned();
        assert_eq!(ref_id, base.token_id());
    }

    #[test]
    fn v0_setters_independent() {
        // Mutating token_count must not touch total_agreed_price, and vice-versa.
        let mut v0 = make_v0(10, 100);
        v0.set_token_count(20);
        assert_eq!(v0.token_count(), 20);
        assert_eq!(v0.total_agreed_price(), 100);

        v0.set_total_agreed_price(500);
        assert_eq!(v0.total_agreed_price(), 500);
        assert_eq!(v0.token_count(), 20);
    }
}
