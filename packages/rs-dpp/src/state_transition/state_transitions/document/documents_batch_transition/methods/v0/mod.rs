use crate::consensus::basic::BasicError::DataContractHaveNewUniqueIndexError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::errors::DataContractError;
use crate::identity::SecurityLevel;
use crate::prelude::DataContract;
use crate::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use crate::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransitionV0Methods;
use crate::state_transition::documents_batch_transition::fields::DEFAULT_SECURITY_LEVEL;
use crate::state_transition::documents_batch_transition::get_security_level_requirement;
use crate::ProtocolError;
use platform_value::Identifier;
use std::convert::TryFrom;

pub trait DocumentsBatchTransitionMethodsV0: DocumentsBatchTransitionAccessorsV0 {
    fn contract_based_security_level_requirement(
        &self,
        get_data_contract_security_level_requirement: impl Fn(
            Identifier,
            String,
        )
            -> Result<SecurityLevel, ProtocolError>,
    ) -> Result<Vec<SecurityLevel>, ProtocolError> {
        // Step 1: Get all document types for the ST
        // Step 2: Get document schema for every type
        // If schema has security level, use that, if not, use the default security level
        // Find the highest level (lowest int value) of all documents - the ST's signature
        // requirement is the highest level across all documents affected by the ST./
        let mut highest_security_level = SecurityLevel::lowest_level();

        for transition in self.transitions().iter() {
            let document_type_name = transition.base().document_type_name();
            let data_contract_id = transition.base().data_contract_id();
            let document_security_level = get_data_contract_security_level_requirement(
                data_contract_id,
                document_type_name.to_owned(),
            )?;

            // lower enum enum representation means higher in security
            if document_security_level < highest_security_level {
                highest_security_level = document_security_level
            }
        }
        Ok(if highest_security_level == SecurityLevel::MASTER {
            vec![SecurityLevel::MASTER]
        } else {
            // this might seem wrong until you realize that master is 0, critical 1, etc
            (SecurityLevel::CRITICAL as u8..=highest_security_level as u8)
                .map(|security_level| SecurityLevel::try_from(security_level).unwrap())
                .collect()
        })
    }
}
