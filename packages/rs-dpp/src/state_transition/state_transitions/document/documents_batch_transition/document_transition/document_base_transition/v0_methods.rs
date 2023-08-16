use crate::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;

use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;

use platform_value::Identifier;

impl DocumentBaseTransitionV0Methods for DocumentBaseTransition {
    fn id(&self) -> Identifier {
        match self {
            DocumentBaseTransition::V0(v0) => v0.id(),
        }
    }

    fn set_id(&mut self, id: Identifier) {
        match self {
            DocumentBaseTransition::V0(v0) => v0.set_id(id),
        }
    }

    fn document_type_name(&self) -> &String {
        match self {
            DocumentBaseTransition::V0(v0) => v0.document_type_name(),
        }
    }

    fn set_document_type_name(&mut self, document_type_name: String) {
        match self {
            DocumentBaseTransition::V0(v0) => v0.set_document_type_name(document_type_name),
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            DocumentBaseTransition::V0(v0) => v0.data_contract_id(),
        }
    }

    fn data_contract_id_ref(&self) -> &Identifier {
        match self {
            DocumentBaseTransition::V0(v0) => v0.data_contract_id_ref(),
        }
    }

    fn set_data_contract_id(&mut self, data_contract_id: Identifier) {
        match self {
            DocumentBaseTransition::V0(v0) => v0.set_data_contract_id(data_contract_id),
        }
    }
}
