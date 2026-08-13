use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{DocumentPropertyReferenceTarget, DocumentPropertyType};
use dpp::data_contract::DataContract;
use dpp::errors::consensus::state::document::referenced_document_type_deletable_error::ReferencedDocumentTypeDeletableError;
use dpp::errors::consensus::state::document::referenced_document_type_not_found_error::ReferencedDocumentTypeNotFoundError;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::query::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};

/// Checks every `permanentDocument` reference declaration of the given
/// contract: the referenced contract must exist (the declaring contract
/// itself when no contract id is named, including when it names its own id),
/// the referenced document type must exist in it, and that type must forbid
/// deletion. Self references are checked against the in-flight contract, so a
/// contract may reference its own document types on creation; foreign contract
/// fetches are billed.
///
/// The error paths name the failing declaration as
/// `documentTypeName.propertyPath`.
pub(super) fn validate_data_contract_references_v0(
    contract: &DataContract,
    drive: &Drive,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    for (declaring_type_name, document_type) in contract.document_types() {
        for (path, property) in document_type.as_ref().flattened_properties() {
            let DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::PermanentDocument {
                    contract_id,
                    document_type_name,
                },
            ) = &property.property_type
            else {
                continue;
            };

            let declaration_path = format!("{declaring_type_name}.{path}");

            let effective_contract_id = contract_id.unwrap_or(contract.id());

            let referenced_contract_fetch_info;
            let referenced_contract = if effective_contract_id == contract.id() {
                contract
            } else {
                let (fee, fetch_info) = drive.get_contract_with_fetch_info_and_fee(
                    effective_contract_id.to_buffer(),
                    Some(&block_info.epoch),
                    false,
                    transaction,
                    platform_version,
                )?;

                let fee = fee.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "fee must exist when fetching a referenced contract with an epoch",
                )))?;

                // The cost is added even if the referenced contract does not exist or was cached
                execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

                let Some(fetch_info) = fetch_info else {
                    // A missing contract and a missing document type resolve to the
                    // same failure: the declared document type could not be found
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ReferencedDocumentTypeNotFoundError::new(
                            effective_contract_id,
                            document_type_name.clone(),
                            declaration_path,
                        )
                        .into(),
                    ));
                };

                referenced_contract_fetch_info = fetch_info;
                &referenced_contract_fetch_info.contract
            };

            let Some(referenced_document_type) =
                referenced_contract.document_type_optional_for_name(document_type_name)
            else {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ReferencedDocumentTypeNotFoundError::new(
                        effective_contract_id,
                        document_type_name.clone(),
                        declaration_path,
                    )
                    .into(),
                ));
            };

            if referenced_document_type.documents_can_be_deleted() {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ReferencedDocumentTypeDeletableError::new(
                        effective_contract_id,
                        document_type_name.clone(),
                        declaration_path,
                    )
                    .into(),
                ));
            }
        }
    }

    Ok(SimpleConsensusValidationResult::new())
}
