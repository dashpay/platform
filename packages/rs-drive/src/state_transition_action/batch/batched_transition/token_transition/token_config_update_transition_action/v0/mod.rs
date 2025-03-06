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
