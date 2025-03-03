mod transformer;

use std::sync::Arc;
use dpp::balances::credits::TokenAmount;
use dpp::identifier::Identifier;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};

/// Token issuance transition action v0
#[derive(Debug, Clone)]
pub struct TokenDestroyFrozenFundsTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// The identity to credit the token to
    pub frozen_identity_id: Identifier,
    /// The amount that will be burned
    pub amount: TokenAmount,
    /// A public note
    pub public_note: Option<String>,
}

/// Accessors for `TokenIssuanceTransitionActionV0`
pub trait TokenDestroyFrozenFundsTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes self and returns the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// Consumes self and returns the identity balance holder ID
    fn frozen_identity_id(&self) -> Identifier;

    /// Sets the identity balance holder ID
    fn set_frozen_identity_id(&mut self, frozen_identity_id: Identifier);

    /// Returns the amount of tokens that the identity had and will be burned
    fn amount(&self) -> TokenAmount;

    /// Sets the amount of tokens that the identity had and will be burned
    fn set_amount(&mut self, amount: TokenAmount);

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

    /// Returns the public note (optional)
    fn public_note(&self) -> Option<&String>;

    /// Returns the public note (owned)
    fn public_note_owned(self) -> Option<String>;

    /// Sets the public note
    fn set_public_note(&mut self, public_note: Option<String>);
}

impl TokenDestroyFrozenFundsTransitionActionAccessorsV0
    for TokenDestroyFrozenFundsTransitionActionV0
{
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn frozen_identity_id(&self) -> Identifier {
        self.frozen_identity_id
    }

    fn set_frozen_identity_id(&mut self, frozen_identity_id: Identifier) {
        self.frozen_identity_id = frozen_identity_id;
    }

    fn amount(&self) -> TokenAmount {
        self.amount
    }

    fn set_amount(&mut self, amount: TokenAmount) {
        self.amount = amount;
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
