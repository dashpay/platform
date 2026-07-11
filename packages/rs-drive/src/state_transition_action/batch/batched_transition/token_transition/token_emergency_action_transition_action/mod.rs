use derive_more::From;
use dpp::tokens::emergency_action::TokenEmergencyAction;

/// transformer module for token emergency_action transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenIssuanceTransitionActionV0)

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token emergency_action transition action
#[derive(Debug, Clone, From)]
pub enum TokenEmergencyActionTransitionAction {
    /// v0
    V0(TokenEmergencyActionTransitionActionV0),
}

impl TokenEmergencyActionTransitionActionAccessorsV0 for TokenEmergencyActionTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenEmergencyActionTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenEmergencyActionTransitionAction::V0(v0) => v0.base,
        }
    }

    fn emergency_action(&self) -> TokenEmergencyAction {
        match self {
            TokenEmergencyActionTransitionAction::V0(v0) => v0.emergency_action(),
        }
    }

    fn set_emergency_action(&mut self, emergency_action: TokenEmergencyAction) {
        match self {
            TokenEmergencyActionTransitionAction::V0(v0) => {
                v0.set_emergency_action(emergency_action)
            }
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenEmergencyActionTransitionAction::V0(v0) => v0.public_note.as_ref(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenEmergencyActionTransitionAction::V0(v0) => v0.public_note,
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenEmergencyActionTransitionAction::V0(v0) => v0.public_note = public_note,
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
            token_id: Identifier::new([0x44; 32]),
            identity_contract_nonce: 13,
            token_contract_position: 0,
            data_contract: Arc::new(fetch_info),
            store_in_group: None,
            perform_action: true,
        })
    }

    fn make_v0(
        action: TokenEmergencyAction,
        note: Option<&str>,
    ) -> TokenEmergencyActionTransitionActionV0 {
        TokenEmergencyActionTransitionActionV0 {
            base: make_base(),
            emergency_action: action,
            public_note: note.map(|s| s.to_string()),
        }
    }

    #[test]
    fn enum_from_v0_yields_v0_variant() {
        let v0 = make_v0(TokenEmergencyAction::Pause, Some("p"));
        let wrapped: TokenEmergencyActionTransitionAction = v0.into();
        assert!(matches!(
            wrapped,
            TokenEmergencyActionTransitionAction::V0(_)
        ));
    }

    #[test]
    fn enum_base_ref_returns_underlying_base() {
        let action =
            TokenEmergencyActionTransitionAction::V0(make_v0(TokenEmergencyAction::Pause, None));
        assert_eq!(action.base().token_id(), Identifier::new([0x44; 32]));
        assert_eq!(action.base().identity_contract_nonce(), 13);
    }

    #[test]
    fn enum_base_owned_consumes_self() {
        let action =
            TokenEmergencyActionTransitionAction::V0(make_v0(TokenEmergencyAction::Resume, None));
        let base = action.base_owned();
        assert_eq!(base.token_id(), Identifier::new([0x44; 32]));
    }

    #[test]
    fn enum_emergency_action_returns_pause() {
        let action =
            TokenEmergencyActionTransitionAction::V0(make_v0(TokenEmergencyAction::Pause, None));
        assert_eq!(action.emergency_action(), TokenEmergencyAction::Pause);
    }

    #[test]
    fn enum_emergency_action_returns_resume() {
        let action =
            TokenEmergencyActionTransitionAction::V0(make_v0(TokenEmergencyAction::Resume, None));
        assert_eq!(action.emergency_action(), TokenEmergencyAction::Resume);
    }

    #[test]
    fn enum_set_emergency_action_swaps_variant() {
        let mut action =
            TokenEmergencyActionTransitionAction::V0(make_v0(TokenEmergencyAction::Pause, None));
        action.set_emergency_action(TokenEmergencyAction::Resume);
        assert_eq!(action.emergency_action(), TokenEmergencyAction::Resume);
        action.set_emergency_action(TokenEmergencyAction::Pause);
        assert_eq!(action.emergency_action(), TokenEmergencyAction::Pause);
    }

    #[test]
    fn enum_public_note_returns_reference_when_set() {
        let action = TokenEmergencyActionTransitionAction::V0(make_v0(
            TokenEmergencyAction::Pause,
            Some("halt"),
        ));
        assert_eq!(action.public_note(), Some(&"halt".to_string()));
    }

    #[test]
    fn enum_public_note_returns_none_when_unset() {
        let action =
            TokenEmergencyActionTransitionAction::V0(make_v0(TokenEmergencyAction::Pause, None));
        assert!(action.public_note().is_none());
    }

    #[test]
    fn enum_public_note_owned_consumes_self_and_returns_inner() {
        let action = TokenEmergencyActionTransitionAction::V0(make_v0(
            TokenEmergencyAction::Pause,
            Some("consumed"),
        ));
        let note = action.public_note_owned();
        assert_eq!(note, Some("consumed".to_string()));
    }

    #[test]
    fn enum_public_note_owned_none_when_unset() {
        let action =
            TokenEmergencyActionTransitionAction::V0(make_v0(TokenEmergencyAction::Resume, None));
        assert!(action.public_note_owned().is_none());
    }

    #[test]
    fn enum_set_public_note_replaces_value() {
        let mut action = TokenEmergencyActionTransitionAction::V0(make_v0(
            TokenEmergencyAction::Pause,
            Some("old"),
        ));
        action.set_public_note(Some("new".to_string()));
        assert_eq!(action.public_note(), Some(&"new".to_string()));
        action.set_public_note(None);
        assert!(action.public_note().is_none());
    }
}
