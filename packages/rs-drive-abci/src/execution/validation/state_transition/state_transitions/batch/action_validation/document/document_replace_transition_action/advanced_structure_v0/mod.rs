use dpp::data_contract::document_type::property_constraints::DocumentSystemValues;
use dpp::consensus::basic::document::{InvalidDocumentTransitionActionError, InvalidDocumentTypeError};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::{DocumentTypeBasicMethods, DocumentTypeV0Methods};
use dpp::identifier::Identifier;
use dpp::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use crate::error::Error;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentReplaceTransitionActionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl DocumentReplaceTransitionActionStructureValidationV0 for DocumentReplaceTransitionAction {
    fn validate_structure_v0(
        &self,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let contract_fetch_info = self.base().data_contract_fetch_info();
        let data_contract = &contract_fetch_info.contract;
        let document_type_name = self.base().document_type_name();

        // Make sure that the document type is defined in the contract
        let Some(document_type) = data_contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), data_contract.id())
                    .into(),
            ));
        };

        if !document_type.documents_mutable() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTransitionActionError::new(format!(
                    "{} is not mutable and can not be replaced",
                    document_type_name
                ))
                .into(),
            ));
        }

        // Validate user defined properties. The rules read the writer, the times and
        // heights the replace keeps (creation, last transfer) and the ones it sets (the
        // update), as the stored document will hold them, and the `countOf` and `sumOf`
        // totals the action read from state as they will be once it is stored.
        let system = DocumentSystemValues {
            owner_id: Some(owner_id),
            created_at: self.created_at(),
            updated_at: self.updated_at(),
            transferred_at: self.transferred_at(),
            created_at_block_height: self.created_at_block_height(),
            updated_at_block_height: self.updated_at_block_height(),
            transferred_at_block_height: self.transferred_at_block_height(),
            created_at_core_block_height: self.created_at_core_block_height(),
            updated_at_core_block_height: self.updated_at_core_block_height(),
            transferred_at_core_block_height: self.transferred_at_core_block_height(),
            aggregates: Some(self.property_constraint_aggregates().clone()),
        };
        let result = data_contract
            .validate_document_properties(
                document_type_name,
                self.data().into(),
                &system,
                platform_version,
            )
            .map_err(Error::Protocol)?;
        if !result.is_valid() {
            return Ok(result);
        }

        // Added in place at protocol version 14, inert for every earlier version this
        // generation serves: their meta-schemas refuse `distinctFrom`, their parser ignores
        // it (`apply_distinct_from` is `None`), and `validate_distinct_from` is `None` there,
        // so the call sees no declaration and returns an empty result. From 14, a
        // `distinctFrom` identifier property must differ from the named sibling property or
        // from the writer's `$ownerId`; both are on the transition, and the schema
        // validation above already made every value compared a 32-byte identifier.
        let result = document_type
            .validate_distinct_from_properties(self.data(), owner_id, platform_version)
            .map_err(Error::Protocol)?;
        if !result.is_valid() {
            return Ok(result);
        }

        // Added in place at protocol version 14, inert for every earlier version this
        // generation serves: their meta-schemas refuse `encryptedFor`, their parser
        // ignores it (`apply_encrypted_for` is `None`, so no parsed property carries a
        // declaration), and `validate_encrypted_property_shapes` is `None` there, so the
        // call returns an empty result. From 14,
        // an `encryptedFor` property the replace supplies must have the shape its scheme
        // produces, which is all consensus can tell about a ciphertext; the schema
        // validation above already made every such value a byte array.
        document_type
            .validate_encrypted_property_shapes(self.data(), platform_version)
            .map_err(Error::Protocol)
    }
}
