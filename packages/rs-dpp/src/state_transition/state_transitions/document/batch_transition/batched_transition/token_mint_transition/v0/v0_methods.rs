use platform_value::Identifier;
use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::ProtocolError;
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::state_transition::batch_transition::token_mint_transition::TokenMintTransitionV0;
use crate::state_transition::batch_transition::TokenMintTransition;
use crate::tokens::errors::TokenError;

impl TokenBaseTransitionAccessors for TokenMintTransitionV0 {
    fn base(&self) -> &TokenBaseTransition {
        &self.base
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        &mut self.base
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        self.base = base;
    }
}

pub trait TokenMintTransitionV0Methods:
    TokenBaseTransitionAccessors + AllowedAsMultiPartyAction
{
    fn amount(&self) -> u64;

    fn set_amount(&mut self, amount: u64);

    /// Returns the `public_note` field of the `TokenMintTransitionV0`.
    fn public_note(&self) -> Option<&String>;

    /// Returns the owned `public_note` field of the `TokenMintTransitionV0`.
    fn public_note_owned(self) -> Option<String>;

    /// Sets the value of the `public_note` field in the `TokenMintTransitionV0`.
    fn set_public_note(&mut self, public_note: Option<String>);

    /// Returns the `issued_to_identity_id` field of the `TokenMintTransitionV0`.
    fn issued_to_identity_id(&self) -> Option<Identifier>;
    fn recipient_id(
        &self,
        token_configuration: &TokenConfiguration,
    ) -> Result<Identifier, ProtocolError>;

    /// Sets the value of the `issued_to_identity_id` field in the `TokenMintTransitionV0`.
    fn set_issued_to_identity_id(&mut self, issued_to_identity_id: Option<Identifier>);
}

impl TokenMintTransitionV0Methods for TokenMintTransitionV0 {
    fn amount(&self) -> u64 {
        self.amount
    }

    fn set_amount(&mut self, amount: u64) {
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

    fn issued_to_identity_id(&self) -> Option<Identifier> {
        self.issued_to_identity_id
    }

    fn recipient_id(
        &self,
        token_configuration: &TokenConfiguration,
    ) -> Result<Identifier, ProtocolError> {
        match self.issued_to_identity_id() {
            None => token_configuration
                .distribution_rules()
                .new_tokens_destination_identity()
                .copied()
                .ok_or(ProtocolError::Token(
                    TokenError::TokenNoMintingRecipient.into(),
                )),
            Some(recipient) => Ok(recipient),
        }
    }

    fn set_issued_to_identity_id(&mut self, issued_to_identity_id: Option<Identifier>) {
        self.issued_to_identity_id = issued_to_identity_id;
    }
}

impl AllowedAsMultiPartyAction for TokenMintTransitionV0 {
    fn calculate_action_id(&self, owner_id: Identifier) -> Identifier {
        let TokenMintTransitionV0 { base, amount, .. } = self;

        TokenMintTransition::calculate_action_id_with_fields(
            base.token_id().as_bytes(),
            owner_id.as_bytes(),
            base.identity_contract_nonce(),
            *amount,
        )
    }
}
