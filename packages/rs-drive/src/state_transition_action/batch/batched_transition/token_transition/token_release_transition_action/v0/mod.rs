mod transformer;

use std::sync::Arc;
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::{TokenDistributionRecipient, TokenDistributionResolvedRecipient};
use dpp::identifier::Identifier;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};

/// Token release transition action v0
#[derive(Debug, Clone)]
pub struct TokenReleaseTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// Amount to be released,
    /// if this is a release to Evonodes or a group, this is the total amount that later needs
    /// to be split up
    pub amount: TokenAmount,
    /// The recipient we wish to release the funds of
    pub recipient: TokenDistributionResolvedRecipient,
    /// The type of distribution we are targeting
    pub distribution_type: TokenDistributionType,
    /// A public note
    pub public_note: Option<String>,
}

/// Accessors for `TokenReleaseTransitionActionV0`
pub trait TokenReleaseTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes self and returns the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

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

    /// Returns the amount to be released
    fn amount(&self) -> TokenAmount;

    /// Sets the amount to be released
    fn set_amount(&mut self, amount: TokenAmount);

    /// Returns the recipient of the distribution
    fn recipient(&self) -> &TokenDistributionResolvedRecipient;

    /// Returns the recipient (owned)
    fn recipient_owned(self) -> TokenDistributionResolvedRecipient;

    /// Sets the recipient
    fn set_recipient(&mut self, recipient: TokenDistributionResolvedRecipient);

    /// Returns the type of distribution
    fn distribution_type(&self) -> TokenDistributionType;

    /// Sets the type of distribution
    fn set_distribution_type(&mut self, distribution_type: TokenDistributionType);

    /// Returns the public note (optional)
    fn public_note(&self) -> Option<&String>;

    /// Returns the public note (owned)
    fn public_note_owned(self) -> Option<String>;

    /// Sets the public note
    fn set_public_note(&mut self, public_note: Option<String>);
}

impl TokenReleaseTransitionActionAccessorsV0 for TokenReleaseTransitionActionV0 {
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn amount(&self) -> TokenAmount {
        self.amount
    }

    fn set_amount(&mut self, amount: TokenAmount) {
        self.amount = amount;
    }

    fn recipient(&self) -> &TokenDistributionResolvedRecipient {
        &self.recipient
    }

    fn recipient_owned(self) -> TokenDistributionResolvedRecipient {
        self.recipient
    }

    fn set_recipient(&mut self, recipient: TokenDistributionResolvedRecipient) {
        self.recipient = recipient;
    }

    fn distribution_type(&self) -> TokenDistributionType {
        self.distribution_type
    }

    fn set_distribution_type(&mut self, distribution_type: TokenDistributionType) {
        self.distribution_type = distribution_type;
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
