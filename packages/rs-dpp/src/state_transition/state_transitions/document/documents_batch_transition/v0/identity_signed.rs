use crate::data_contract::document_schema::DataContractDocumentSchemaMethodsV0;
use crate::identity::{KeyID, SecurityLevel};
use crate::prelude::DataContract;
use crate::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransitionV0Methods;
use crate::state_transition::documents_batch_transition::fields::DEFAULT_SECURITY_LEVEL;
use crate::state_transition::documents_batch_transition::{
    get_security_level_requirement, DocumentsBatchTransitionV0,
};
use crate::state_transition::StateTransitionIdentitySigned;
use crate::ProtocolError;
use std::convert::TryFrom;

impl StateTransitionIdentitySigned for DocumentsBatchTransitionV0 {
    fn signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = key_id
    }

    fn security_level_requirement(&self) -> Vec<SecurityLevel> {
        //should use contract_based_security_level_requirement instead
        unreachable!()
    }
}
