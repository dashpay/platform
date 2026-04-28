mod transformer;

use std::sync::Arc;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dpp::data_contract::TokenContractPosition;
use dpp::identifier::Identifier;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};

/// Token config update transition action v0
#[derive(Debug, Clone)]
pub struct TokenConfigUpdateTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// Updated token configuration item
    pub update_token_configuration_item: TokenConfigurationChangeItem,
    /// A public note
    pub public_note: Option<String>,
}

/// Accessors for `TokenIssuanceTransitionActionV0`
pub trait TokenConfigUpdateTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes self and returns the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// Returns the `update_token_configuration_item`
    fn update_token_configuration_item(&self) -> &TokenConfigurationChangeItem;

    /// Sets the `update_token_configuration_item`
    fn set_update_token_configuration_item(
        &mut self,
        update_token_configuration_item: TokenConfigurationChangeItem,
    );

    /// Returns the token position in the contract
    fn token_position(&self) -> TokenContractPosition {
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

impl TokenConfigUpdateTransitionActionAccessorsV0 for TokenConfigUpdateTransitionActionV0 {
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn update_token_configuration_item(&self) -> &TokenConfigurationChangeItem {
        &self.update_token_configuration_item
    }

    fn set_update_token_configuration_item(
        &mut self,
        update_token_configuration_item: TokenConfigurationChangeItem,
    ) {
        self.update_token_configuration_item = update_token_configuration_item;
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
    use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
    use dpp::version::PlatformVersion;

    fn make_base() -> TokenBaseTransitionAction {
        let fetch_info = DataContractFetchInfo::dpns_contract_fixture(
            PlatformVersion::latest().protocol_version,
        );
        TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
            token_id: Identifier::new([0xBE; 32]),
            identity_contract_nonce: 8,
            token_contract_position: 0,
            data_contract: Arc::new(fetch_info),
            store_in_group: None,
            perform_action: true,
        })
    }

    fn make_v0_with(
        item: TokenConfigurationChangeItem,
        note: Option<&str>,
    ) -> TokenConfigUpdateTransitionActionV0 {
        TokenConfigUpdateTransitionActionV0 {
            base: make_base(),
            update_token_configuration_item: item,
            public_note: note.map(|s| s.to_string()),
        }
    }

    #[test]
    fn v0_update_token_configuration_item_returns_ref() {
        let v0 = make_v0_with(TokenConfigurationChangeItem::MaxSupply(Some(300)), None);
        match v0.update_token_configuration_item() {
            TokenConfigurationChangeItem::MaxSupply(Some(v)) => assert_eq!(*v, 300),
            other => panic!("unexpected variant: {:?}", other),
        }
    }

    #[test]
    fn v0_set_update_token_configuration_item_replaces_value() {
        let mut v0 = make_v0_with(TokenConfigurationChangeItem::MaxSupply(Some(1)), None);
        v0.set_update_token_configuration_item(
            TokenConfigurationChangeItem::TokenConfigurationNoChange,
        );
        assert!(matches!(
            v0.update_token_configuration_item(),
            TokenConfigurationChangeItem::TokenConfigurationNoChange
        ));
    }

    #[test]
    fn v0_set_update_supports_authorized_action_takers_variant() {
        let mut v0 = make_v0_with(TokenConfigurationChangeItem::MaxSupply(None), None);
        v0.set_update_token_configuration_item(TokenConfigurationChangeItem::ManualMinting(
            AuthorizedActionTakers::ContractOwner,
        ));
        match v0.update_token_configuration_item() {
            TokenConfigurationChangeItem::ManualMinting(AuthorizedActionTakers::ContractOwner) => {}
            other => panic!("unexpected variant: {:?}", other),
        }
    }

    #[test]
    fn v0_public_note_accessors() {
        let v0 = make_v0_with(
            TokenConfigurationChangeItem::TokenConfigurationNoChange,
            Some("note"),
        );
        assert_eq!(v0.public_note(), Some(&"note".to_string()));
        let owned = v0.public_note_owned();
        assert_eq!(owned, Some("note".to_string()));
    }

    #[test]
    fn v0_set_public_note_round_trip() {
        let mut v0 = make_v0_with(
            TokenConfigurationChangeItem::TokenConfigurationNoChange,
            None,
        );
        assert!(v0.public_note().is_none());
        v0.set_public_note(Some("x".to_string()));
        assert_eq!(v0.public_note(), Some(&"x".to_string()));
        v0.set_public_note(None);
        assert!(v0.public_note().is_none());
    }

    #[test]
    fn v0_default_accessors_delegate_to_base() {
        let v0 = make_v0_with(
            TokenConfigurationChangeItem::TokenConfigurationNoChange,
            None,
        );
        assert_eq!(v0.token_position(), 0);
        assert_eq!(v0.token_id(), Identifier::new([0xBE; 32]));
        let fetch = v0.data_contract_fetch_info();
        assert_eq!(v0.data_contract_id(), fetch.contract.id());
        assert!(Arc::ptr_eq(
            v0.data_contract_fetch_info_ref(),
            &v0.data_contract_fetch_info(),
        ));
    }

    #[test]
    fn v0_base_owned_preserves_token_id() {
        let v0 = make_v0_with(
            TokenConfigurationChangeItem::TokenConfigurationNoChange,
            None,
        );
        let id_from_ref = v0.base().token_id();
        let base = v0.base_owned();
        assert_eq!(id_from_ref, base.token_id());
    }
}
