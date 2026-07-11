use derive_more::From;
use dpp::identifier::Identifier;

/// transformer module for token freeze transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenIssuanceTransitionActionV0)

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token freeze transition action
#[derive(Debug, Clone, From)]
pub enum TokenUnfreezeTransitionAction {
    /// v0
    V0(TokenUnfreezeTransitionActionV0),
}

impl TokenUnfreezeTransitionActionAccessorsV0 for TokenUnfreezeTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenUnfreezeTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenUnfreezeTransitionAction::V0(v0) => v0.base,
        }
    }

    fn frozen_identity_id(&self) -> Identifier {
        match self {
            TokenUnfreezeTransitionAction::V0(v0) => v0.frozen_identity_id,
        }
    }

    fn set_frozen_identity_id(&mut self, id: Identifier) {
        match self {
            TokenUnfreezeTransitionAction::V0(v0) => v0.frozen_identity_id = id,
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenUnfreezeTransitionAction::V0(v0) => v0.public_note.as_ref(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenUnfreezeTransitionAction::V0(v0) => v0.public_note,
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenUnfreezeTransitionAction::V0(v0) => v0.public_note = public_note,
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
            token_id: Identifier::new([0x33; 32]),
            identity_contract_nonce: 21,
            token_contract_position: 0,
            data_contract: Arc::new(fetch_info),
            store_in_group: None,
            perform_action: true,
        })
    }

    fn make_v0(frozen_id: Identifier, note: Option<&str>) -> TokenUnfreezeTransitionActionV0 {
        TokenUnfreezeTransitionActionV0 {
            base: make_base(),
            frozen_identity_id: frozen_id,
            public_note: note.map(|s| s.to_string()),
        }
    }

    #[test]
    fn enum_from_v0_wraps_in_v0_variant() {
        let v0 = make_v0(Identifier::new([0xA; 32]), Some("note"));
        let wrapped: TokenUnfreezeTransitionAction = v0.into();
        assert!(matches!(wrapped, TokenUnfreezeTransitionAction::V0(_)));
    }

    #[test]
    fn enum_base_returns_underlying_base() {
        let action = TokenUnfreezeTransitionAction::V0(make_v0(Identifier::new([0xAA; 32]), None));
        assert_eq!(action.base().token_id(), Identifier::new([0x33; 32]));
        assert_eq!(action.base().identity_contract_nonce(), 21);
    }

    #[test]
    fn enum_base_owned_consumes_self_and_returns_base() {
        let action =
            TokenUnfreezeTransitionAction::V0(make_v0(Identifier::new([0xBB; 32]), Some("bye")));
        let base = action.base_owned();
        assert_eq!(base.token_id(), Identifier::new([0x33; 32]));
    }

    #[test]
    fn enum_frozen_identity_id_returns_stored_value() {
        let id = Identifier::new([0xEE; 32]);
        let action = TokenUnfreezeTransitionAction::V0(make_v0(id, None));
        assert_eq!(action.frozen_identity_id(), id);
    }

    #[test]
    fn enum_set_frozen_identity_id_mutates_inner() {
        let mut action =
            TokenUnfreezeTransitionAction::V0(make_v0(Identifier::new([0x11; 32]), None));
        let new_id = Identifier::new([0x99; 32]);
        action.set_frozen_identity_id(new_id);
        assert_eq!(action.frozen_identity_id(), new_id);
    }

    #[test]
    fn enum_public_note_returns_reference_when_set() {
        let action = TokenUnfreezeTransitionAction::V0(make_v0(
            Identifier::new([0x10; 32]),
            Some("thawing"),
        ));
        assert_eq!(action.public_note(), Some(&"thawing".to_string()));
    }

    #[test]
    fn enum_public_note_returns_none_when_unset() {
        let action = TokenUnfreezeTransitionAction::V0(make_v0(Identifier::new([0x10; 32]), None));
        assert!(action.public_note().is_none());
    }

    #[test]
    fn enum_public_note_owned_consumes_self_and_returns_note() {
        let action = TokenUnfreezeTransitionAction::V0(make_v0(
            Identifier::new([0x10; 32]),
            Some("consumed"),
        ));
        let owned = action.public_note_owned();
        assert_eq!(owned, Some("consumed".to_string()));
    }

    #[test]
    fn enum_public_note_owned_returns_none_when_unset() {
        let action = TokenUnfreezeTransitionAction::V0(make_v0(Identifier::new([0x10; 32]), None));
        assert!(action.public_note_owned().is_none());
    }

    #[test]
    fn enum_set_public_note_replaces_and_clears() {
        let mut action =
            TokenUnfreezeTransitionAction::V0(make_v0(Identifier::new([0x10; 32]), Some("old")));
        action.set_public_note(Some("newer".to_string()));
        assert_eq!(action.public_note(), Some(&"newer".to_string()));
        action.set_public_note(None);
        assert!(action.public_note().is_none());
    }
}
