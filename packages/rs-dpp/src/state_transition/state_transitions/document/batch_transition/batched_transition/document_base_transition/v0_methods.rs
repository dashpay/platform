use crate::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;

use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;

use crate::prelude::IdentityNonce;
use platform_value::Identifier;

impl DocumentBaseTransitionV0Methods for DocumentBaseTransition {
    fn id(&self) -> Identifier {
        match self {
            DocumentBaseTransition::V0(v0) => v0.id(),
            DocumentBaseTransition::V1(v1) => v1.id(),
        }
    }

    fn set_id(&mut self, id: Identifier) {
        match self {
            DocumentBaseTransition::V0(v0) => v0.set_id(id),
            DocumentBaseTransition::V1(v1) => v1.set_id(id),
        }
    }

    fn document_type_name(&self) -> &String {
        match self {
            DocumentBaseTransition::V0(v0) => v0.document_type_name(),
            DocumentBaseTransition::V1(v1) => v1.document_type_name(),
        }
    }
    fn document_type_name_owned(self) -> String {
        match self {
            DocumentBaseTransition::V0(v0) => v0.document_type_name_owned(),
            DocumentBaseTransition::V1(v1) => v1.document_type_name_owned(),
        }
    }

    fn set_document_type_name(&mut self, document_type_name: String) {
        match self {
            DocumentBaseTransition::V0(v0) => v0.set_document_type_name(document_type_name),
            DocumentBaseTransition::V1(v1) => v1.set_document_type_name(document_type_name),
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            DocumentBaseTransition::V0(v0) => v0.data_contract_id(),
            DocumentBaseTransition::V1(v1) => v1.data_contract_id(),
        }
    }

    fn data_contract_id_ref(&self) -> &Identifier {
        match self {
            DocumentBaseTransition::V0(v0) => v0.data_contract_id_ref(),
            DocumentBaseTransition::V1(v1) => v1.data_contract_id_ref(),
        }
    }

    fn set_data_contract_id(&mut self, data_contract_id: Identifier) {
        match self {
            DocumentBaseTransition::V0(v0) => v0.set_data_contract_id(data_contract_id),
            DocumentBaseTransition::V1(v1) => v1.set_data_contract_id(data_contract_id),
        }
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            DocumentBaseTransition::V0(v0) => v0.identity_contract_nonce,
            DocumentBaseTransition::V1(v1) => v1.identity_contract_nonce,
        }
    }

    fn set_identity_contract_nonce(&mut self, identity_contract_nonce: IdentityNonce) {
        match self {
            DocumentBaseTransition::V0(v0) => v0.identity_contract_nonce = identity_contract_nonce,
            DocumentBaseTransition::V1(v1) => v1.identity_contract_nonce = identity_contract_nonce,
        }
    }
}
