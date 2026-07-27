use derive_more::From;
use dpp::identifier::Identifier;

/// transformer module for token freeze transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenIssuanceTransitionActionV0)

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token freeze transition action
#[derive(Debug, Clone, From)]
pub enum TokenFreezeTransitionAction {
    /// v0
    V0(TokenFreezeTransitionActionV0),
}

impl TokenFreezeTransitionActionAccessorsV0 for TokenFreezeTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenFreezeTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenFreezeTransitionAction::V0(v0) => v0.base,
        }
    }

    fn identity_to_freeze_id(&self) -> Identifier {
        match self {
            TokenFreezeTransitionAction::V0(v0) => v0.identity_to_freeze_id,
        }
    }

    fn set_identity_to_freeze_id(&mut self, id: Identifier) {
        match self {
            TokenFreezeTransitionAction::V0(v0) => v0.identity_to_freeze_id = id,
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenFreezeTransitionAction::V0(v0) => v0.public_note.as_ref(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenFreezeTransitionAction::V0(v0) => v0.public_note,
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenFreezeTransitionAction::V0(v0) => v0.public_note = public_note,
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
    use dpp::version::PlatformVersion;
    use std::sync::Arc;

    fn make_base() -> TokenBaseTransitionAction {
        let fetch_info = DataContractFetchInfo::dpns_contract_fixture(
            PlatformVersion::latest().protocol_version,
        );
        TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
            token_id: Identifier::new([0x55; 32]),
            identity_contract_nonce: 30,
            token_contract_position: 0,
            data_contract: Arc::new(fetch_info),
            store_in_group: None,
            perform_action: true,
        })
    }

    fn make_v0(target: Identifier, note: Option<&str>) -> TokenFreezeTransitionActionV0 {
        TokenFreezeTransitionActionV0 {
            base: make_base(),
            identity_to_freeze_id: target,
            public_note: note.map(|s| s.to_string()),
        }
    }

    #[test]
    fn enum_from_v0_wraps_in_v0_variant() {
        let v0 = make_v0(Identifier::new([0xA; 32]), None);
        let wrapped: TokenFreezeTransitionAction = v0.into();
        assert!(matches!(wrapped, TokenFreezeTransitionAction::V0(_)));
    }

    #[test]
    fn enum_base_returns_underlying_base() {
        let action = TokenFreezeTransitionAction::V0(make_v0(Identifier::new([0xAA; 32]), None));
        assert_eq!(action.base().token_id(), Identifier::new([0x55; 32]));
        assert_eq!(action.base().identity_contract_nonce(), 30);
    }

    #[test]
    fn enum_base_owned_consumes_self_and_returns_base() {
        let action =
            TokenFreezeTransitionAction::V0(make_v0(Identifier::new([0xBB; 32]), Some("bye")));
        let base = action.base_owned();
        assert_eq!(base.token_id(), Identifier::new([0x55; 32]));
    }

    #[test]
    fn enum_identity_to_freeze_id_returns_stored_value() {
        let id = Identifier::new([0xEE; 32]);
        let action = TokenFreezeTransitionAction::V0(make_v0(id, None));
        assert_eq!(action.identity_to_freeze_id(), id);
    }

    #[test]
    fn enum_set_identity_to_freeze_id_mutates_inner() {
        let mut action =
            TokenFreezeTransitionAction::V0(make_v0(Identifier::new([0x11; 32]), None));
        let new_id = Identifier::new([0x99; 32]);
        action.set_identity_to_freeze_id(new_id);
        assert_eq!(action.identity_to_freeze_id(), new_id);
    }

    #[test]
    fn enum_public_note_returns_reference_when_set() {
        let action =
            TokenFreezeTransitionAction::V0(make_v0(Identifier::new([0x10; 32]), Some("freezing")));
        assert_eq!(action.public_note(), Some(&"freezing".to_string()));
    }

    #[test]
    fn enum_public_note_returns_none_when_unset() {
        let action = TokenFreezeTransitionAction::V0(make_v0(Identifier::new([0x10; 32]), None));
        assert!(action.public_note().is_none());
    }

    #[test]
    fn enum_public_note_owned_consumes_self_and_returns_note() {
        let action =
            TokenFreezeTransitionAction::V0(make_v0(Identifier::new([0x10; 32]), Some("consumed")));
        let owned = action.public_note_owned();
        assert_eq!(owned, Some("consumed".to_string()));
    }

    #[test]
    fn enum_public_note_owned_returns_none_when_unset() {
        let action = TokenFreezeTransitionAction::V0(make_v0(Identifier::new([0x10; 32]), None));
        assert!(action.public_note_owned().is_none());
    }

    #[test]
    fn enum_set_public_note_replaces_and_clears() {
        let mut action =
            TokenFreezeTransitionAction::V0(make_v0(Identifier::new([0x10; 32]), Some("old")));
        action.set_public_note(Some("newer".to_string()));
        assert_eq!(action.public_note(), Some(&"newer".to_string()));
        action.set_public_note(None);
        assert!(action.public_note().is_none());
    }
}
