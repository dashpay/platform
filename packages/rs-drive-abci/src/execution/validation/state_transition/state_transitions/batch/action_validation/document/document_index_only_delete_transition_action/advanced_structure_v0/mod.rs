use dpp::consensus::basic::document::{
    InvalidDocumentTransitionActionError, InvalidDocumentTypeError,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
use dpp::document::property_names::CREATED_AT;
use dpp::prelude::TimestampMillis;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_index_only_delete_transition_action::v0::DocumentIndexOnlyDeleteTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_index_only_delete_transition_action::DocumentIndexOnlyDeleteTransitionAction;

use crate::error::Error;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentIndexOnlyDeleteTransitionActionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl DocumentIndexOnlyDeleteTransitionActionStructureValidationV0
    for DocumentIndexOnlyDeleteTransitionAction
{
    fn validate_structure_v0(
        &self,
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

        if !document_type.documents_can_be_deleted() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTransitionActionError::new(format!(
                    "documents of type {} can not be deleted",
                    document_type_name
                ))
                .into(),
            ));
        }

        // Pair the delete KIND with the doctype's storage mode: a stored
        // document is deleted by id (the delete kind), and its values
        // would be nothing this pipeline validates against — the mirror
        // of the delete kind's refusal of indexOnly types.
        if !document_type.index_only() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTransitionActionError::new(format!(
                    "documents of stored type {} must be deleted with a delete (by-id) \
                     transition; indexOnlyDelete is only for indexOnly types",
                    document_type_name
                ))
                .into(),
            ));
        }

        // The values are untrusted and directly select the storage entries
        // the state layer probes and the operation conversion removes, so
        // they must satisfy the document schema BEFORE anything derives an
        // index key from them — a malformed value must die here as a
        // consensus error, never later as an internal error.
        //
        // `$createdAt` rides in `data` under its system key (the user
        // schema knows nothing of it): split it off, pin its presence to
        // the doctype's requirement (indexed `$createdAt` forces it into
        // `required`, and it feeds the row commitment either way, so a
        // spurious or missing value could only ever produce probe
        // mismatches downstream), and type-check it as a timestamp.
        let mut user_data = self.data().clone();
        let carried_created_at = user_data.remove(CREATED_AT);
        let requires_created_at = document_type.required_fields().contains(CREATED_AT);
        match (requires_created_at, carried_created_at) {
            (true, None) => {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    InvalidDocumentTransitionActionError::new(format!(
                        "a delete of indexOnly document type {} must carry $createdAt: the \
                         type requires it and it is part of every entry's row commitment",
                        document_type_name
                    ))
                    .into(),
                ));
            }
            (false, Some(_)) => {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    InvalidDocumentTransitionActionError::new(format!(
                        "a delete of indexOnly document type {} must not carry $createdAt: \
                         the type does not use it",
                        document_type_name
                    ))
                    .into(),
                ));
            }
            (true, Some(value)) => {
                if value.to_integer::<TimestampMillis>().is_err() {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidDocumentTransitionActionError::new(format!(
                            "a delete of indexOnly document type {} carries a $createdAt \
                             that is not a timestamp",
                            document_type_name
                        ))
                        .into(),
                    ));
                }
            }
            (false, None) => {}
        }

        // The remaining map is exactly a document's user properties —
        // validate it with the same contract validator creates use, which
        // enforces required properties, value types, and rejects unknown
        // keys (system fields included, since the user schema admits none).
        data_contract
            .validate_document_properties(document_type_name, user_data.into(), platform_version)
            .map_err(Error::Protocol)
    }
}
