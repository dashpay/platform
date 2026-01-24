use std::collections::{BTreeMap, BTreeSet};

use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::consensus::basic::invalid_identifier_error::InvalidIdentifierError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::document_type::property::reference::DocumentPropertyReferenceTarget;
use dpp::errors::consensus::state::document::referenced_entity_not_found_error::ReferencedEntityNotFoundError;
use dpp::identifier::Identifier;
use dpp::platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use dpp::platform_value::Value;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;

use crate::error::Error;
use crate::execution::types::execution_operation::{RetrieveIdentityInfo, ValidationOperation};
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::platform_types::platform::PlatformStateRef;

/// Versioned, stateful validation of document references using the v0 rules.
///
/// This performs existence checks for supported reference targets (currently identity) and
/// can be limited to changed fields for replace transitions. It is intended to be called via
/// the higher-level `DocumentReferenceValidation` dispatcher that selects the version.
pub(crate) trait DocumentReferenceValidationV0 {
    fn validate_document_references_v0(
        &self,
        document_data: &BTreeMap<String, Value>,
        changed_fields: Option<&BTreeSet<String>>,
        platform: &PlatformStateRef,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentReferenceValidationV0 for DocumentBaseTransitionAction {
    fn validate_document_references_v0(
        &self,
        document_data: &BTreeMap<String, Value>,
        changed_fields: Option<&BTreeSet<String>>,
        platform: &PlatformStateRef,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let contract_fetch_info = self.data_contract_fetch_info();
        let contract = &contract_fetch_info.contract;
        let document_type_name = self.document_type_name();

        let Some(document_type) = contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), contract.id()).into(),
            ));
        };

        validate_document_type_references_v0(
            document_type,
            document_data,
            changed_fields,
            platform,
            transaction,
            execution_context,
            platform_version,
        )
    }
}

fn validate_document_type_references_v0(
    document_type: DocumentTypeRef<'_>,
    document_data: &BTreeMap<String, Value>,
    changed_fields: Option<&BTreeSet<String>>,
    platform: &PlatformStateRef,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    for (path, property) in document_type.flattened_properties() {
        if let Some(changed) = changed_fields {
            if !changed.contains(path) {
                continue;
            }
        }

        let Some(reference) = &property.reference else {
            continue;
        };

        match reference.target {
            DocumentPropertyReferenceTarget::IdentityReferenceTarget => {
                let validation_result = validate_identity_reference_v0(
                    document_data,
                    path,
                    reference.must_exist,
                    platform,
                    transaction,
                    execution_context,
                    platform_version,
                )?;
                if !validation_result.is_valid() {
                    return Ok(validation_result);
                }
            }
            _ => continue,
        }
    }

    Ok(SimpleConsensusValidationResult::new())
}

fn validate_identity_reference_v0(
    document_data: &BTreeMap<String, Value>,
    path: &str,
    must_exist: bool,
    platform: &PlatformStateRef,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    if !must_exist {
        return Ok(SimpleConsensusValidationResult::new());
    }

    let identity_bytes = match document_data.get_optional_identifier_at_path(path) {
        Ok(Some(identity_bytes)) => identity_bytes,
        Ok(None) => {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidIdentifierError::new(path.to_string(), "not set".to_string()).into(),
            ))
        }
        Err(err) => {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidIdentifierError::new(path.to_string(), err.to_string()).into(),
            ))
        }
    };

    execution_context.add_operation(ValidationOperation::RetrieveIdentity(
        RetrieveIdentityInfo::only_revision(),
    ));

    let Some(_) = platform.drive.fetch_identity_revision(
        identity_bytes,
        true,
        transaction,
        platform_version,
    )?
    else {
        let missing_id =
            Identifier::from_bytes(&identity_bytes).map_err(|e| Error::Protocol(e.into()))?;

        return Ok(SimpleConsensusValidationResult::new_with_error(
            ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(
                ReferencedEntityNotFoundError::new(
                    missing_id,
                    DocumentPropertyReferenceTarget::IdentityReferenceTarget,
                    path.to_string(),
                ),
            )),
        ));
    };

    Ok(SimpleConsensusValidationResult::new())
}
