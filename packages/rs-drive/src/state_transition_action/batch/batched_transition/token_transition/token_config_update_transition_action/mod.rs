use derive_more::From;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;

/// transformer module for token freeze transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenIssuanceTransitionActionV0)

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token config update transition action
#[derive(Debug, Clone, From)]
pub enum TokenConfigUpdateTransitionAction {
    /// v0
    V0(TokenConfigUpdateTransitionActionV0),
}

impl TokenConfigUpdateTransitionActionAccessorsV0 for TokenConfigUpdateTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenConfigUpdateTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenConfigUpdateTransitionAction::V0(v0) => v0.base,
        }
    }

    fn update_token_configuration_item(&self) -> &TokenConfigurationChangeItem {
        match self {
            TokenConfigUpdateTransitionAction::V0(v0) => &v0.update_token_configuration_item,
        }
    }

    fn set_update_token_configuration_item(
        &mut self,
        update_token_configuration_item: TokenConfigurationChangeItem,
    ) {
        match self {
            TokenConfigUpdateTransitionAction::V0(v0) => {
                v0.update_token_configuration_item = update_token_configuration_item
            }
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenConfigUpdateTransitionAction::V0(v0) => v0.public_note.as_ref(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenConfigUpdateTransitionAction::V0(v0) => v0.public_note,
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenConfigUpdateTransitionAction::V0(v0) => v0.public_note = public_note,
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
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;
    use std::sync::Arc;

    const TEST_OWNER_ID: [u8; 32] = [0xAA; 32];
    const TEST_TOKEN_ID: [u8; 32] = [0xCC; 32];
    const TEST_NONCE: u64 = 5;
    const TEST_TOKEN_POSITION: u16 = 0;

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

    fn make_v0_with_item(
        item: TokenConfigurationChangeItem,
    ) -> TokenConfigUpdateTransitionActionV0 {
        TokenConfigUpdateTransitionActionV0 {
            base: make_base_action(),
            update_token_configuration_item: item,
            public_note: Some("cfg-update".to_string()),
        }
    }

    fn make_v0() -> TokenConfigUpdateTransitionActionV0 {
        make_v0_with_item(TokenConfigurationChangeItem::MaxSupply(Some(1_000)))
    }

    #[test]
    fn from_v0_wraps_in_enum() {
        let v0 = make_v0();
        let action: TokenConfigUpdateTransitionAction = v0.into();
        assert!(matches!(action, TokenConfigUpdateTransitionAction::V0(_)));
    }

    #[test]
    fn enum_base_returns_v0_base() {
        let action = TokenConfigUpdateTransitionAction::V0(make_v0());
        let base = action.base();
        assert_eq!(base.token_id(), Identifier::from(TEST_TOKEN_ID));
        assert_eq!(base.identity_contract_nonce(), TEST_NONCE);
        assert_eq!(base.token_position(), TEST_TOKEN_POSITION);
    }

    #[test]
    fn enum_base_owned_consumes() {
        let action = TokenConfigUpdateTransitionAction::V0(make_v0());
        let base = action.base_owned();
        assert_eq!(base.token_id(), Identifier::from(TEST_TOKEN_ID));
    }

    #[test]
    fn enum_update_token_configuration_item_returns_ref() {
        let action = TokenConfigUpdateTransitionAction::V0(make_v0());
        match action.update_token_configuration_item() {
            TokenConfigurationChangeItem::MaxSupply(Some(v)) => assert_eq!(*v, 1_000),
            other => panic!("unexpected item variant: {:?}", other),
        }
    }

    #[test]
    fn enum_set_update_token_configuration_item_replaces_value() {
        let mut action = TokenConfigUpdateTransitionAction::V0(make_v0());
        action.set_update_token_configuration_item(
            TokenConfigurationChangeItem::TokenConfigurationNoChange,
        );
        assert!(matches!(
            action.update_token_configuration_item(),
            TokenConfigurationChangeItem::TokenConfigurationNoChange
        ));
        // Replace again with a different variant to exercise overwrite.
        action.set_update_token_configuration_item(TokenConfigurationChangeItem::MaxSupply(None));
        assert!(matches!(
            action.update_token_configuration_item(),
            TokenConfigurationChangeItem::MaxSupply(None)
        ));
    }

    #[test]
    fn enum_public_note_returns_ref() {
        let action = TokenConfigUpdateTransitionAction::V0(make_v0());
        assert_eq!(action.public_note(), Some(&"cfg-update".to_string()));
    }

    #[test]
    fn enum_public_note_returns_none_when_unset() {
        let mut v0 = make_v0();
        v0.public_note = None;
        let action = TokenConfigUpdateTransitionAction::V0(v0);
        assert_eq!(action.public_note(), None);
    }

    #[test]
    fn enum_public_note_owned_consumes() {
        let action = TokenConfigUpdateTransitionAction::V0(make_v0());
        assert_eq!(action.public_note_owned(), Some("cfg-update".to_string()));
    }

    #[test]
    fn enum_set_public_note_replaces_value() {
        let mut action = TokenConfigUpdateTransitionAction::V0(make_v0());
        action.set_public_note(Some("newer".to_string()));
        assert_eq!(action.public_note(), Some(&"newer".to_string()));
        action.set_public_note(None);
        assert_eq!(action.public_note(), None);
    }

    #[test]
    fn v0_accessors_roundtrip() {
        let mut v0 = make_v0();
        assert!(matches!(
            v0.update_token_configuration_item(),
            TokenConfigurationChangeItem::MaxSupply(Some(1_000))
        ));
        assert_eq!(v0.public_note(), Some(&"cfg-update".to_string()));

        v0.set_update_token_configuration_item(
            TokenConfigurationChangeItem::TokenConfigurationNoChange,
        );
        assert!(matches!(
            v0.update_token_configuration_item(),
            TokenConfigurationChangeItem::TokenConfigurationNoChange
        ));

        v0.set_public_note(None);
        assert_eq!(v0.public_note(), None);
        v0.set_public_note(Some("set-again".to_string()));
        assert_eq!(v0.public_note(), Some(&"set-again".to_string()));

        let owned = v0.clone().public_note_owned();
        assert_eq!(owned, Some("set-again".to_string()));

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

    #[test]
    fn v0_stores_various_change_item_variants() {
        // Exercise a few TokenConfigurationChangeItem variants to ensure the
        // field is held and retrievable by reference for several kinds of
        // payloads the transformer might receive.
        let v_nochange =
            make_v0_with_item(TokenConfigurationChangeItem::TokenConfigurationNoChange);
        assert!(matches!(
            v_nochange.update_token_configuration_item(),
            TokenConfigurationChangeItem::TokenConfigurationNoChange
        ));

        let v_max_none = make_v0_with_item(TokenConfigurationChangeItem::MaxSupply(None));
        assert!(matches!(
            v_max_none.update_token_configuration_item(),
            TokenConfigurationChangeItem::MaxSupply(None)
        ));

        let v_max_some = make_v0_with_item(TokenConfigurationChangeItem::MaxSupply(Some(12_345)));
        match v_max_some.update_token_configuration_item() {
            TokenConfigurationChangeItem::MaxSupply(Some(v)) => assert_eq!(*v, 12_345),
            other => panic!("unexpected variant: {:?}", other),
        }
    }
}
