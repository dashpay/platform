use crate::prelude::IdentityNonce;
use crate::state_transition::documents_batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::state_transition::documents_batch_transition::token_base_transition::TokenBaseTransition;
use platform_value::Identifier;

impl TokenBaseTransitionV0Methods for TokenBaseTransition {
    fn token_contract_position(&self) -> u16 {
        match self {
            TokenBaseTransition::V0(v0) => v0.token_contract_position(),
        }
    }

    fn set_token_contract_position(&mut self, token_id: u16) {
        match self {
            TokenBaseTransition::V0(v0) => v0.set_token_contract_position(token_id),
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            TokenBaseTransition::V0(v0) => v0.data_contract_id(),
        }
    }

    fn data_contract_id_ref(&self) -> &Identifier {
        match self {
            TokenBaseTransition::V0(v0) => v0.data_contract_id_ref(),
        }
    }

    fn set_data_contract_id(&mut self, data_contract_id: Identifier) {
        match self {
            TokenBaseTransition::V0(v0) => v0.set_data_contract_id(data_contract_id),
        }
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            TokenBaseTransition::V0(v0) => v0.identity_contract_nonce,
        }
    }

    fn set_identity_contract_nonce(&mut self, identity_contract_nonce: IdentityNonce) {
        match self {
            TokenBaseTransition::V0(v0) => v0.identity_contract_nonce = identity_contract_nonce,
        }
    }
}
