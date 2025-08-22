mod transformer;

use std::sync::Arc;
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionInfo;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
use dpp::identifier::Identifier;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};

/// Token release transition action v0
#[derive(Debug, Clone)]
pub struct TokenClaimTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// Amount to be released,
    /// if this is a release to Evonodes or a group, this is the total amount that later needs
    /// to be split up
    pub amount: TokenAmount,
    /// The type of distribution we are targeting
    pub distribution_info: TokenDistributionInfo,
    /// A public note
    pub public_note: Option<String>,
}

/// Accessors for `TokenClaimTransitionActionV0`
pub trait TokenClaimTransitionActionAccessorsV0 {
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
    fn recipient(&self) -> TokenDistributionRecipient;

    /// Returns the type of distribution with its recipient
    fn distribution_info(&self) -> &TokenDistributionInfo;

    /// Sets the type of distribution with its recipient
    fn set_distribution_info(&mut self, distribution_type: TokenDistributionInfo);

    /// Returns the public note (optional)
    fn public_note(&self) -> Option<&String>;

    /// Returns the public note (owned)
    fn public_note_owned(self) -> Option<String>;

    /// Sets the public note
    fn set_public_note(&mut self, public_note: Option<String>);
}

impl TokenClaimTransitionActionAccessorsV0 for TokenClaimTransitionActionV0 {
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

    fn recipient(&self) -> TokenDistributionRecipient {
        match &self.distribution_info {
            TokenDistributionInfo::PreProgrammed(_, identifier) => {
                TokenDistributionRecipient::Identity(*identifier)
            }
            TokenDistributionInfo::Perpetual(_, resolved_recipient) => resolved_recipient.into(),
        }
    }

    fn distribution_info(&self) -> &TokenDistributionInfo {
        &self.distribution_info
    }

    fn set_distribution_info(&mut self, distribution_info: TokenDistributionInfo) {
        self.distribution_info = distribution_info;
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
