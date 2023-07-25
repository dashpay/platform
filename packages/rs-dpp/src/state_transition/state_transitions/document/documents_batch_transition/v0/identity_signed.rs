use crate::data_contract::document_schema::DataContractDocumentSchemaMethodsV0;
use crate::identity::{KeyID, SecurityLevel};
use crate::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransitionV0Methods;
use crate::state_transition::documents_batch_transition::fields::DEFAULT_SECURITY_LEVEL;
use crate::state_transition::documents_batch_transition::{
    get_security_level_requirement, DocumentsBatchTransitionV0,
};
use crate::state_transition::StateTransitionIdentitySigned;
use platform_value::Identifier;
use std::convert::TryFrom;

impl StateTransitionIdentitySigned for DocumentsBatchTransitionV0 {
    fn signature_public_key_id(&self) -> Option<KeyID> {
        Some(self.signature_public_key_id)
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = key_id
    }

    fn security_level_requirement(&self) -> Vec<SecurityLevel> {
        // Step 1: Get all document types for the ST
        // Step 2: Get document schema for every type
        // If schema has security level, use that, if not, use the default security level
        // Find the highest level (lowest int value) of all documents - the ST's signature
        // requirement is the highest level across all documents affected by the ST./
        let mut highest_security_level = SecurityLevel::lowest_level();

        for transition in self.transitions.iter() {
            let document_type = &transition.base().document_type_name();
            let data_contract = &transition.base().data_contract();
            let maybe_document_schema = data_contract.get_document_schema(document_type);

            if let Ok(document_schema) = maybe_document_schema {
                let document_security_level =
                    get_security_level_requirement(document_schema, DEFAULT_SECURITY_LEVEL);

                // lower enum enum representation means higher in security
                if document_security_level < highest_security_level {
                    highest_security_level = document_security_level
                }
            }
        }
        if highest_security_level == SecurityLevel::MASTER {
            vec![SecurityLevel::MASTER]
        } else {
            (SecurityLevel::CRITICAL as u8..=highest_security_level as u8)
                .map(|security_level| SecurityLevel::try_from(security_level).unwrap())
                .collect()
        }
    }
}
