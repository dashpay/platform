use platform_value::Identifier;
use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::prelude::IdentityNonce;
use crate::ProtocolError;
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_mint_transition::TokenMintTransition;
use crate::state_transition::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use crate::util::hash::hash_double;

impl TokenBaseTransitionAccessors for TokenMintTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenMintTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenMintTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenMintTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenMintTransitionV0Methods for TokenMintTransition {
    fn amount(&self) -> u64 {
        match self {
            TokenMintTransition::V0(v0) => v0.amount(),
        }
    }

    fn set_amount(&mut self, amount: u64) {
        match self {
            TokenMintTransition::V0(v0) => v0.set_amount(amount),
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenMintTransition::V0(v0) => v0.public_note(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenMintTransition::V0(v0) => v0.public_note_owned(),
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenMintTransition::V0(v0) => v0.set_public_note(public_note),
        }
    }

    fn issued_to_identity_id(&self) -> Option<Identifier> {
        match self {
            TokenMintTransition::V0(v0) => v0.issued_to_identity_id(),
        }
    }

    fn recipient_id(
        &self,
        token_configuration: &TokenConfiguration,
    ) -> Result<Identifier, ProtocolError> {
        match self {
            TokenMintTransition::V0(v0) => v0.recipient_id(token_configuration),
        }
    }

    fn set_issued_to_identity_id(&mut self, issued_to_identity_id: Option<Identifier>) {
        match self {
            TokenMintTransition::V0(v0) => v0.set_issued_to_identity_id(issued_to_identity_id),
        }
    }
}

impl AllowedAsMultiPartyAction for TokenMintTransition {
    fn calculate_action_id(&self, owner_id: Identifier) -> Identifier {
        match self {
            TokenMintTransition::V0(v0) => v0.calculate_action_id(owner_id),
        }
    }
}

impl TokenMintTransition {
    pub fn calculate_action_id_with_fields(
        token_id: &[u8; 32],
        owner_id: &[u8; 32],
        identity_contract_nonce: IdentityNonce,
        mint_amount: TokenAmount,
    ) -> Identifier {
        let mut bytes = b"action_token_mint".to_vec();
        bytes.extend_from_slice(token_id);
        bytes.extend_from_slice(owner_id);
        bytes.extend_from_slice(&identity_contract_nonce.to_be_bytes());
        bytes.extend_from_slice(&mint_amount.to_be_bytes());

        hash_double(bytes).into()
    }
}
