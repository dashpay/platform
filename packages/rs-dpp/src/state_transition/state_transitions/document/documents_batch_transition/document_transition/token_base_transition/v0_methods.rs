use crate::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;

use crate::state_transition::documents_batch_transition::document_base_transition::TokenBaseTransition;

use crate::prelude::IdentityNonce;
use platform_value::Identifier;

impl TokenBaseTransitionV0Methods for TokenBaseTransition {
    fn id(&self) -> Identifier {
        match self {
            TokenBaseTransition::V0(v0) => v0.id(),
        }
    }

    fn set_id(&mut self, id: Identifier) {
        match self {
            TokenBaseTransition::V0(v0) => v0.set_id(id),
        }
    }

    fn document_type_name(&self) -> &String {
        match self {
            TokenBaseTransition::V0(v0) => v0.document_type_name(),
        }
    }

    fn set_document_type_name(&mut self, document_type_name: String) {
        match self {
            TokenBaseTransition::V0(v0) => v0.set_document_type_name(document_type_name),
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
