use derive_more::From;
use dpp::balances::credits::TokenAmount;
use dpp::identifier::Identifier;

/// transformer module for token destroy_frozen_funds transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenIssuanceTransitionActionV0)

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token destroy_frozen_funds transition action
#[derive(Debug, Clone, From)]
pub enum TokenDestroyFrozenFundsTransitionAction {
    /// v0
    V0(TokenDestroyFrozenFundsTransitionActionV0),
}

impl TokenDestroyFrozenFundsTransitionActionAccessorsV0
    for TokenDestroyFrozenFundsTransitionAction
{
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenDestroyFrozenFundsTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenDestroyFrozenFundsTransitionAction::V0(v0) => v0.base,
        }
    }

    fn frozen_identity_id(&self) -> Identifier {
        match self {
            TokenDestroyFrozenFundsTransitionAction::V0(v0) => v0.frozen_identity_id,
        }
    }

    fn set_frozen_identity_id(&mut self, id: Identifier) {
        match self {
            TokenDestroyFrozenFundsTransitionAction::V0(v0) => v0.frozen_identity_id = id,
        }
    }

    fn amount(&self) -> TokenAmount {
        match self {
            TokenDestroyFrozenFundsTransitionAction::V0(v0) => v0.amount,
        }
    }

    fn set_amount(&mut self, amount: TokenAmount) {
        match self {
            TokenDestroyFrozenFundsTransitionAction::V0(v0) => v0.amount = amount,
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenDestroyFrozenFundsTransitionAction::V0(v0) => v0.public_note.as_ref(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenDestroyFrozenFundsTransitionAction::V0(v0) => v0.public_note,
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenDestroyFrozenFundsTransitionAction::V0(v0) => v0.public_note = public_note,
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
    const TEST_FROZEN_ID: [u8; 32] = [0xEE; 32];
    const TEST_NONCE: u64 = 4;
    const TEST_TOKEN_POSITION: u16 = 0;
    const TEST_AMOUNT: TokenAmount = 999;

    fn make_base_action() -> TokenBaseTransitionAction {
        let platform_version = PlatformVersion::latest();
        let contract = get_data_contract_fixture(
            Some(Identifier::from(TEST_OWNER_ID)),
            0,
            platform_version.protocol_version,
        )
        .data_contract_owned();
        let fetch_info = Arc::new(DataContractFetchInfo {
            contract,
            storage_flags: None,
            cost: Default::default(),
            fee: None,
        });
        TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
            token_id: Identifier::from(TEST_TOKEN_ID),
            identity_contract_nonce: TEST_NONCE,
            token_contract_position: TEST_TOKEN_POSITION,
            data_contract: fetch_info,
            store_in_group: None,
            perform_action: true,
        })
    }

    fn make_v0() -> TokenDestroyFrozenFundsTransitionActionV0 {
        TokenDestroyFrozenFundsTransitionActionV0 {
            base: make_base_action(),
            frozen_identity_id: Identifier::from(TEST_FROZEN_ID),
            amount: TEST_AMOUNT,
            public_note: Some("destroyed".to_string()),
        }
    }

    #[test]
    fn from_v0_wraps_in_enum() {
        let v0 = make_v0();
        let action: TokenDestroyFrozenFundsTransitionAction = v0.into();
        assert!(matches!(
            action,
            TokenDestroyFrozenFundsTransitionAction::V0(_)
        ));
    }

    #[test]
    fn enum_base_returns_v0_base() {
        let action = TokenDestroyFrozenFundsTransitionAction::V0(make_v0());
        let base = action.base();
        assert_eq!(base.token_id(), Identifier::from(TEST_TOKEN_ID));
        assert_eq!(base.identity_contract_nonce(), TEST_NONCE);
        assert_eq!(base.token_position(), TEST_TOKEN_POSITION);
    }

    #[test]
    fn enum_base_owned_consumes() {
        let action = TokenDestroyFrozenFundsTransitionAction::V0(make_v0());
        let base = action.base_owned();
        assert_eq!(base.token_id(), Identifier::from(TEST_TOKEN_ID));
    }

    #[test]
    fn enum_frozen_identity_id_returns_v0_value() {
        let action = TokenDestroyFrozenFundsTransitionAction::V0(make_v0());
        assert_eq!(
            action.frozen_identity_id(),
            Identifier::from(TEST_FROZEN_ID)
        );
    }

    #[test]
    fn enum_set_frozen_identity_id_updates_v0() {
        let mut action = TokenDestroyFrozenFundsTransitionAction::V0(make_v0());
        let new_id = Identifier::from([0x22; 32]);
        action.set_frozen_identity_id(new_id);
        assert_eq!(action.frozen_identity_id(), new_id);
    }

    #[test]
    fn enum_amount_returns_v0_amount() {
        let action = TokenDestroyFrozenFundsTransitionAction::V0(make_v0());
        assert_eq!(action.amount(), TEST_AMOUNT);
    }

    #[test]
    fn enum_set_amount_updates_v0() {
        let mut action = TokenDestroyFrozenFundsTransitionAction::V0(make_v0());
        action.set_amount(42);
        assert_eq!(action.amount(), 42);
        action.set_amount(0);
        assert_eq!(action.amount(), 0);
    }

    #[test]
    fn enum_public_note_returns_ref() {
        let action = TokenDestroyFrozenFundsTransitionAction::V0(make_v0());
        assert_eq!(action.public_note(), Some(&"destroyed".to_string()));
    }

    #[test]
    fn enum_public_note_returns_none_when_unset() {
        let mut v0 = make_v0();
        v0.public_note = None;
        let action = TokenDestroyFrozenFundsTransitionAction::V0(v0);
        assert_eq!(action.public_note(), None);
    }

    #[test]
    fn enum_public_note_owned_consumes() {
        let action = TokenDestroyFrozenFundsTransitionAction::V0(make_v0());
        assert_eq!(action.public_note_owned(), Some("destroyed".to_string()));
    }

    #[test]
    fn enum_set_public_note_replaces_value() {
        let mut action = TokenDestroyFrozenFundsTransitionAction::V0(make_v0());
        action.set_public_note(Some("revised".to_string()));
        assert_eq!(action.public_note(), Some(&"revised".to_string()));
        action.set_public_note(None);
        assert_eq!(action.public_note(), None);
    }

    #[test]
    fn v0_accessors_roundtrip() {
        let mut v0 = make_v0();
        assert_eq!(v0.amount(), TEST_AMOUNT);
        assert_eq!(v0.frozen_identity_id(), Identifier::from(TEST_FROZEN_ID));
        assert_eq!(v0.public_note(), Some(&"destroyed".to_string()));

        v0.set_amount(3);
        assert_eq!(v0.amount(), 3);

        let new_id = Identifier::from([0x33; 32]);
        v0.set_frozen_identity_id(new_id);
        assert_eq!(v0.frozen_identity_id(), new_id);

        v0.set_public_note(None);
        assert_eq!(v0.public_note(), None);
        v0.set_public_note(Some("again".to_string()));
        assert_eq!(v0.public_note(), Some(&"again".to_string()));

        let owned = v0.clone().public_note_owned();
        assert_eq!(owned, Some("again".to_string()));

        let base = v0.base_owned();
        assert_eq!(base.token_id(), Identifier::from(TEST_TOKEN_ID));
    }

    #[test]
    fn v0_base_delegations() {
        let v0 = make_v0();
        assert_eq!(v0.token_position(), TEST_TOKEN_POSITION);
        assert_eq!(v0.token_id(), Identifier::from(TEST_TOKEN_ID));
        let fetched_arc = v0.data_contract_fetch_info();
        let fetched_ref = v0.data_contract_fetch_info_ref();
        assert!(Arc::ptr_eq(&fetched_arc, fetched_ref));
        assert_eq!(v0.data_contract_id(), fetched_ref.contract.id());
    }
}
