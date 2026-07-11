use derive_more::From;
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;

/// transformer module for token issuance transition action
pub mod transformer;
mod v0;

pub use v0::*;

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token issuance transition action
#[derive(Debug, Clone, From)]
pub enum TokenDirectPurchaseTransitionAction {
    /// v0
    V0(TokenDirectPurchaseTransitionActionV0),
}

impl TokenDirectPurchaseTransitionActionAccessorsV0 for TokenDirectPurchaseTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenDirectPurchaseTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenDirectPurchaseTransitionAction::V0(v0) => v0.base,
        }
    }
    fn token_count(&self) -> TokenAmount {
        match self {
            TokenDirectPurchaseTransitionAction::V0(v0) => v0.token_count,
        }
    }

    fn set_token_count(&mut self, amount: TokenAmount) {
        match self {
            TokenDirectPurchaseTransitionAction::V0(v0) => v0.token_count = amount,
        }
    }

    fn total_agreed_price(&self) -> Credits {
        match self {
            TokenDirectPurchaseTransitionAction::V0(v0) => v0.total_agreed_price,
        }
    }

    fn set_total_agreed_price(&mut self, agreed_price: Credits) {
        match self {
            TokenDirectPurchaseTransitionAction::V0(v0) => v0.total_agreed_price = agreed_price,
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
    use dpp::identifier::Identifier;
    use dpp::version::PlatformVersion;
    use std::sync::Arc;

    fn make_base() -> TokenBaseTransitionAction {
        let fetch_info = DataContractFetchInfo::dpns_contract_fixture(
            PlatformVersion::latest().protocol_version,
        );
        TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
            token_id: Identifier::new([0x22; 32]),
            identity_contract_nonce: 4,
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
    fn enum_from_v0_wraps_in_v0_variant() {
        let wrapped: TokenDirectPurchaseTransitionAction = make_v0(5, 500).into();
        assert!(matches!(
            wrapped,
            TokenDirectPurchaseTransitionAction::V0(_)
        ));
    }

    #[test]
    fn enum_base_returns_underlying_base() {
        let action = TokenDirectPurchaseTransitionAction::V0(make_v0(1, 1));
        assert_eq!(action.base().token_id(), Identifier::new([0x22; 32]));
        assert_eq!(action.base().identity_contract_nonce(), 4);
    }

    #[test]
    fn enum_base_owned_consumes_self_and_returns_base() {
        let action = TokenDirectPurchaseTransitionAction::V0(make_v0(2, 2));
        let base = action.base_owned();
        assert_eq!(base.token_id(), Identifier::new([0x22; 32]));
    }

    #[test]
    fn enum_token_count_returns_stored_amount() {
        let action = TokenDirectPurchaseTransitionAction::V0(make_v0(42, 100));
        assert_eq!(action.token_count(), 42);
    }

    #[test]
    fn enum_set_token_count_mutates_inner() {
        let mut action = TokenDirectPurchaseTransitionAction::V0(make_v0(0, 0));
        action.set_token_count(9_999);
        assert_eq!(action.token_count(), 9_999);
        action.set_token_count(0);
        assert_eq!(action.token_count(), 0);
    }

    #[test]
    fn enum_total_agreed_price_returns_stored_price() {
        let action = TokenDirectPurchaseTransitionAction::V0(make_v0(1, 12_345));
        assert_eq!(action.total_agreed_price(), 12_345);
    }

    #[test]
    fn enum_set_total_agreed_price_mutates_inner() {
        let mut action = TokenDirectPurchaseTransitionAction::V0(make_v0(1, 10));
        action.set_total_agreed_price(1_000_000);
        assert_eq!(action.total_agreed_price(), 1_000_000);
    }

    #[test]
    fn enum_set_token_count_independent_of_total_agreed_price() {
        let mut action = TokenDirectPurchaseTransitionAction::V0(make_v0(2, 200));
        action.set_token_count(5);
        assert_eq!(action.token_count(), 5);
        // Price should remain unchanged.
        assert_eq!(action.total_agreed_price(), 200);
    }

    #[test]
    fn enum_set_total_agreed_price_independent_of_token_count() {
        let mut action = TokenDirectPurchaseTransitionAction::V0(make_v0(2, 200));
        action.set_total_agreed_price(500);
        assert_eq!(action.total_agreed_price(), 500);
        // Token count should remain unchanged.
        assert_eq!(action.token_count(), 2);
    }

    #[test]
    fn v0_accessors_roundtrip() {
        let mut v0 = make_v0(10, 100);
        assert_eq!(v0.token_count(), 10);
        assert_eq!(v0.total_agreed_price(), 100);

        v0.set_token_count(7);
        assert_eq!(v0.token_count(), 7);

        v0.set_total_agreed_price(777);
        assert_eq!(v0.total_agreed_price(), 777);
    }

    #[test]
    fn v0_default_accessors_delegate_to_base() {
        let v0 = make_v0(1, 1);
        assert_eq!(v0.token_position(), 0);
        assert_eq!(v0.token_id(), Identifier::new([0x22; 32]));
        assert!(Arc::ptr_eq(
            v0.data_contract_fetch_info_ref(),
            &v0.data_contract_fetch_info(),
        ));
    }

    #[test]
    fn v0_base_owned_preserves_token_id() {
        let v0 = make_v0(1, 1);
        let id_from_ref = v0.base().token_id();
        let base = v0.base_owned();
        assert_eq!(id_from_ref, base.token_id());
    }
}
