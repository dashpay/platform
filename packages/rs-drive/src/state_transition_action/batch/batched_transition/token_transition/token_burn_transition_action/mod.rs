use derive_more::From;
use dpp::identifier::Identifier;

/// transformer module for token burn transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenBurnTransitionActionV0)

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token burn transition action
#[derive(Debug, Clone, From)]
pub enum TokenBurnTransitionAction {
    /// v0
    V0(TokenBurnTransitionActionV0),
}

impl TokenBurnTransitionActionAccessorsV0 for TokenBurnTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenBurnTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenBurnTransitionAction::V0(v0) => v0.base,
        }
    }

    fn burn_from_identifier(&self) -> Identifier {
        match self {
            TokenBurnTransitionAction::V0(v0) => v0.burn_from_identifier,
        }
    }

    fn set_burn_from_identifier(&mut self, burn_from_identifier: Identifier) {
        match self {
            TokenBurnTransitionAction::V0(v0) => v0.burn_from_identifier = burn_from_identifier,
        }
    }

    fn burn_amount(&self) -> u64 {
        match self {
            TokenBurnTransitionAction::V0(v0) => v0.burn_amount,
        }
    }

    fn set_burn_amount(&mut self, amount: u64) {
        match self {
            TokenBurnTransitionAction::V0(v0) => v0.burn_amount = amount,
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenBurnTransitionAction::V0(v0) => v0.public_note.as_ref(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenBurnTransitionAction::V0(v0) => v0.public_note,
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenBurnTransitionAction::V0(v0) => v0.public_note = public_note,
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
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;
    use std::sync::Arc;

    const TEST_OWNER_ID: [u8; 32] = [0xAA; 32];
    const TEST_TOKEN_ID: [u8; 32] = [0xCC; 32];
    const TEST_BURN_FROM_ID: [u8; 32] = [0xDD; 32];
    const TEST_NONCE: u64 = 9;
    const TEST_TOKEN_POSITION: u16 = 0;
    const TEST_BURN_AMOUNT: u64 = 12_345;

    fn make_base_action() -> TokenBaseTransitionAction {
        let platform_version = PlatformVersion::latest();
        let contract = get_data_contract_fixture(
            Some(Identifier::from(TEST_OWNER_ID)),
            0,
            platform_version.protocol_version,
        )
        .data_contract_owned();
        let contract_id = contract.id();
        let fetch_info = Arc::new(DataContractFetchInfo {
            contract,
            storage_flags: None,
            cost: Default::default(),
            fee: None,
        });
        let v0 = TokenBaseTransitionActionV0 {
            token_id: Identifier::from(TEST_TOKEN_ID),
            identity_contract_nonce: TEST_NONCE,
            token_contract_position: TEST_TOKEN_POSITION,
            data_contract: fetch_info,
            store_in_group: None,
            perform_action: true,
        };
        let _ = contract_id; // keep for potential future assertions
        TokenBaseTransitionAction::V0(v0)
    }

    fn make_v0() -> TokenBurnTransitionActionV0 {
        TokenBurnTransitionActionV0 {
            base: make_base_action(),
            burn_from_identifier: Identifier::from(TEST_BURN_FROM_ID),
            burn_amount: TEST_BURN_AMOUNT,
            public_note: Some("initial note".to_string()),
        }
    }

    #[test]
    fn from_v0_wraps_in_enum() {
        let v0 = make_v0();
        let action: TokenBurnTransitionAction = v0.into();
        assert!(matches!(action, TokenBurnTransitionAction::V0(_)));
    }

    #[test]
    fn enum_base_returns_v0_base() {
        let action = TokenBurnTransitionAction::V0(make_v0());
        let base = action.base();
        assert_eq!(base.token_id(), Identifier::from(TEST_TOKEN_ID));
        assert_eq!(base.identity_contract_nonce(), TEST_NONCE);
        assert_eq!(base.token_position(), TEST_TOKEN_POSITION);
    }

    #[test]
    fn enum_base_owned_consumes_and_returns_base() {
        let action = TokenBurnTransitionAction::V0(make_v0());
        let base = action.base_owned();
        assert_eq!(base.token_id(), Identifier::from(TEST_TOKEN_ID));
    }

    #[test]
    fn enum_burn_from_identifier_returns_v0_value() {
        let action = TokenBurnTransitionAction::V0(make_v0());
        assert_eq!(
            action.burn_from_identifier(),
            Identifier::from(TEST_BURN_FROM_ID)
        );
    }

    #[test]
    fn enum_set_burn_from_identifier_updates_v0() {
        let mut action = TokenBurnTransitionAction::V0(make_v0());
        let new_id = Identifier::from([0xFE; 32]);
        action.set_burn_from_identifier(new_id);
        assert_eq!(action.burn_from_identifier(), new_id);
    }

    #[test]
    fn enum_burn_amount_returns_v0_amount() {
        let action = TokenBurnTransitionAction::V0(make_v0());
        assert_eq!(action.burn_amount(), TEST_BURN_AMOUNT);
    }

    #[test]
    fn enum_set_burn_amount_updates_v0() {
        let mut action = TokenBurnTransitionAction::V0(make_v0());
        action.set_burn_amount(99);
        assert_eq!(action.burn_amount(), 99);
        // Also round-trip with zero
        action.set_burn_amount(0);
        assert_eq!(action.burn_amount(), 0);
    }

    #[test]
    fn enum_public_note_returns_ref() {
        let action = TokenBurnTransitionAction::V0(make_v0());
        assert_eq!(action.public_note(), Some(&"initial note".to_string()));
    }

    #[test]
    fn enum_public_note_when_none() {
        let mut v0 = make_v0();
        v0.public_note = None;
        let action = TokenBurnTransitionAction::V0(v0);
        assert_eq!(action.public_note(), None);
    }

    #[test]
    fn enum_public_note_owned_consumes_and_returns_string() {
        let action = TokenBurnTransitionAction::V0(make_v0());
        assert_eq!(action.public_note_owned(), Some("initial note".to_string()));
    }

    #[test]
    fn enum_set_public_note_replaces_value() {
        let mut action = TokenBurnTransitionAction::V0(make_v0());
        action.set_public_note(Some("updated".to_string()));
        assert_eq!(action.public_note(), Some(&"updated".to_string()));
        action.set_public_note(None);
        assert_eq!(action.public_note(), None);
    }

    // ---- V0 struct accessors ----

    #[test]
    fn v0_accessors_roundtrip() {
        let mut v0 = make_v0();
        assert_eq!(v0.burn_amount(), TEST_BURN_AMOUNT);
        assert_eq!(
            v0.burn_from_identifier(),
            Identifier::from(TEST_BURN_FROM_ID)
        );
        assert_eq!(v0.public_note(), Some(&"initial note".to_string()));

        v0.set_burn_amount(7);
        assert_eq!(v0.burn_amount(), 7);

        let new_id = Identifier::from([0x11; 32]);
        v0.set_burn_from_identifier(new_id);
        assert_eq!(v0.burn_from_identifier(), new_id);

        v0.set_public_note(Some("new".to_string()));
        assert_eq!(v0.public_note(), Some(&"new".to_string()));

        // public_note_owned moves self
        let owned = v0.clone().public_note_owned();
        assert_eq!(owned, Some("new".to_string()));

        // base_owned moves self
        let base = v0.base_owned();
        assert_eq!(base.token_id(), Identifier::from(TEST_TOKEN_ID));
    }

    #[test]
    fn v0_base_ref_and_delegations() {
        let v0 = make_v0();
        // Accessor methods from the trait that delegate to base()
        assert_eq!(v0.token_position(), TEST_TOKEN_POSITION);
        assert_eq!(v0.token_id(), Identifier::from(TEST_TOKEN_ID));
        assert_eq!(v0.identity_contract_nonce(), TEST_NONCE);
        let fetched_arc = v0.data_contract_fetch_info();
        let fetched_ref = v0.data_contract_fetch_info_ref();
        assert!(Arc::ptr_eq(&fetched_arc, fetched_ref));
        assert_eq!(v0.data_contract_id(), fetched_ref.contract.id());
    }
}
