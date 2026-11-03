use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::ProtocolError;
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::state_transition::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use crate::state_transition::batch_transition::token_mint_transition::TokenMintTransitionV1;
use crate::state_transition::batch_transition::TokenMintTransition;
use crate::tokens::errors::TokenError;

impl TokenBaseTransitionAccessors for TokenMintTransitionV1 {
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

impl TokenMintTransitionV0Methods for TokenMintTransitionV1 {
    fn amount(&self) -> u64 {
        self.amount
    }

    fn set_amount(&mut self, amount: u64) {
        self.amount = amount;
    }

    fn random_minimum_amount(&self) -> Option<u64> {
        self.random_minimum_amount
    }

    fn set_random_minimum_amount(&mut self, random_minimum_amount: Option<u64>) {
        self.random_minimum_amount = random_minimum_amount;
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

impl AllowedAsMultiPartyAction for TokenMintTransitionV1 {
    /// The same id a version 0 mint of `amount` has. A random mint is never a group action
    /// (structure validation refuses one), so the random minimum amount takes no part.
    fn calculate_action_id(
        &self,
        owner_id: Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Identifier, ProtocolError> {
        let TokenMintTransitionV1 { base, amount, .. } = self;

        Ok(TokenMintTransition::calculate_action_id_with_fields(
            base.token_id().as_bytes(),
            owner_id.as_bytes(),
            base.identity_contract_nonce(),
            *amount,
        ))
    }
}
