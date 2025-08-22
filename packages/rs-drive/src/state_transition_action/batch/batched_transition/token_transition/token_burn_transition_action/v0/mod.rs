mod transformer;

use std::sync::Arc;
use dpp::balances::credits::TokenAmount;
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};

/// Token burn transition action v0
#[derive(Debug, Clone)]
pub struct TokenBurnTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// Burn from identifier
    pub burn_from_identifier: Identifier,
    /// The amount of tokens to burn
    pub burn_amount: TokenAmount,
    /// A public note
    pub public_note: Option<String>,
}

/// Accessors for `TokenBurnTransitionActionV0`
pub trait TokenBurnTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes self and returns the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// Returns the identifier of the identity account from which we will burn
    fn burn_from_identifier(&self) -> Identifier;

    /// Sets the identifier of the identity account from which we will burn
    fn set_burn_from_identifier(&mut self, burn_from_identifier: Identifier);

    /// Returns the amount of tokens to burn
    fn burn_amount(&self) -> u64;

    /// Sets the amount of tokens to burn
    fn set_burn_amount(&mut self, amount: u64);

    /// Returns a reference to the `public_note` field of the `TokenBurnTransitionActionV0`
    fn public_note(&self) -> Option<&String>;

    /// Returns the owned `public_note` field of the `TokenBurnTransitionActionV0`
    fn public_note_owned(self) -> Option<String>;

    /// Sets the value of the `public_note` field in the `TokenBurnTransitionActionV0`
    fn set_public_note(&mut self, public_note: Option<String>);

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

    /// Returns the identity contract nonce
    fn identity_contract_nonce(&self) -> IdentityNonce {
        self.base().identity_contract_nonce()
    }
}

impl TokenBurnTransitionActionAccessorsV0 for TokenBurnTransitionActionV0 {
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn burn_from_identifier(&self) -> Identifier {
        self.burn_from_identifier
    }

    fn set_burn_from_identifier(&mut self, burn_from_identifier: Identifier) {
        self.burn_from_identifier = burn_from_identifier;
    }

    fn burn_amount(&self) -> u64 {
        self.burn_amount
    }

    fn set_burn_amount(&mut self, amount: u64) {
        self.burn_amount = amount;
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
