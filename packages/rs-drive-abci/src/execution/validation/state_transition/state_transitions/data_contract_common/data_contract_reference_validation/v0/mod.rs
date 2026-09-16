use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{DocumentPropertyReferenceTarget, DocumentPropertyType};
use dpp::data_contract::DataContract;
use dpp::errors::consensus::state::document::referenced_document_property_agreement_invalid_error::ReferencedDocumentPropertyAgreementInvalidError;
use dpp::errors::consensus::state::document::referenced_document_type_deletable_error::ReferencedDocumentTypeDeletableError;
use dpp::errors::consensus::state::document::referenced_document_type_not_found_error::ReferencedDocumentTypeNotFoundError;
use dpp::errors::consensus::state::document::referenced_key_id_property_invalid_error::ReferencedKeyIdPropertyInvalidError;
use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::contract::DataContractFetchInfo;
use drive::drive::Drive;
use drive::query::TransactionArg;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};

/// Checks every reference declaration of the given contract that carries
/// declaration content.
///
/// `permanentDocument`: the referenced contract must exist (the declaring
/// contract itself when no contract id is named, including when it names its
/// own id), the referenced document type must exist in it, and that type must
/// forbid deletion. Self references are checked against the in-flight
/// contract, so a contract may reference its own document types on creation;
/// foreign contract fetches are billed.
///
/// `identityPublicKey`: the declared key id property must exist in the same
/// document type and be an integer.
///
/// The error paths name the failing declaration as
/// `documentTypeName.propertyPath`. Validation stops at the first invalid
/// Whether two property types hold the same KIND of value for agreement
/// purposes: sizes and other constraints may differ (both sides validated
/// their own documents already), and an identifier is one kind whether or
/// not it carries its own reference annotation.
fn same_value_kind(a: &DocumentPropertyType, b: &DocumentPropertyType) -> bool {
    let normalized_kind = |property_type: &DocumentPropertyType| match property_type {
        DocumentPropertyType::Identifier | DocumentPropertyType::IdentifierWithReference(_) => {
            std::mem::discriminant(&DocumentPropertyType::Identifier)
        }
        other => std::mem::discriminant(other),
    };
    normalized_kind(a) == normalized_kind(b)
}

/// declaration: this bounds the billed work an invalid contract can cause and
/// matches document write-time reference validation. Foreign contract
/// resolutions are memoized per contract id, so a contract declaring many
/// references into the same foreign contract is billed one fetch for it.
pub(super) fn validate_data_contract_references_v0(
    contract: &DataContract,
    drive: &Drive,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    // Memoizes foreign contract resolutions (including misses) so repeated
    // declarations naming the same contract are billed a single fetch
    let mut fetched_contracts: BTreeMap<Identifier, Option<Arc<DataContractFetchInfo>>> =
        BTreeMap::new();

    for (declaring_type_name, document_type) in contract.document_types() {
        for (path, property) in document_type.as_ref().flattened_properties() {
            let DocumentPropertyType::IdentifierWithReference(reference_target) =
                &property.property_type
            else {
                continue;
            };

            let declaration_path = format!("{declaring_type_name}.{path}");

            // The key id property must exist in the same document type and be
            // an integer; nothing else about the declaration is state-dependent
            if let DocumentPropertyReferenceTarget::IdentityPublicKey { key_id_property } =
                reference_target
            {
                match document_type
                    .as_ref()
                    .flattened_properties()
                    .get(key_id_property)
                {
                    None => {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ReferencedKeyIdPropertyInvalidError::new(
                                key_id_property.clone(),
                                declaration_path,
                                "the document type does not define this property".to_string(),
                            )
                            .into(),
                        ));
                    }
                    Some(key_property) if !key_property.property_type.is_integer() => {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ReferencedKeyIdPropertyInvalidError::new(
                                key_id_property.clone(),
                                declaration_path,
                                "the property must be an integer".to_string(),
                            )
                            .into(),
                        ));
                    }
                    Some(_) => continue,
                }
            }

            let DocumentPropertyReferenceTarget::PermanentDocument {
                contract_id,
                document_type_name,
                property_agreement,
            } = reference_target
            else {
                continue;
            };

            let effective_contract_id = contract_id.unwrap_or(contract.id());

            let referenced_contract_fetch_info;
            let referenced_contract = if effective_contract_id == contract.id() {
                contract
            } else {
                let resolved = match fetched_contracts.get(&effective_contract_id) {
                    Some(cached) => cached.clone(),
                    None => {
                        let (fee, fetch_info) = drive.get_contract_with_fetch_info_and_fee(
                            effective_contract_id.to_buffer(),
                            Some(&block_info.epoch),
                            false,
                            transaction,
                            platform_version,
                        )?;

                        let fee =
                            fee.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                                "fee must exist when fetching a referenced contract with an epoch",
                            )))?;

                        // The cost is added even if the referenced contract does not exist
                        // or was served from Drive's own contract cache; only locally
                        // memoized repeats above skip it
                        execution_context
                            .add_operation(ValidationOperation::PrecalculatedOperation(fee));

                        fetched_contracts.insert(effective_contract_id, fetch_info.clone());

                        fetch_info
                    }
                };

                let Some(fetch_info) = resolved else {
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

            // propertyAgreement declarations: both sides must exist, be
            // plain values (not containers), and share one value kind — a
            // cross-kind equality could never be satisfied and would brick
            // every create of the declaring document type.
            for (referring_property, referenced_property) in property_agreement {
                let invalid = |reason: &str| {
                    SimpleConsensusValidationResult::new_with_error(
                        ReferencedDocumentPropertyAgreementInvalidError::new(
                            declaration_path.clone(),
                            referring_property.clone(),
                            referenced_property.clone(),
                            reason.to_string(),
                        )
                        .into(),
                    )
                };
                if referring_property == path {
                    return Ok(invalid(
                        "the referring property cannot be the reference property itself",
                    ));
                }
                let declaring_document_type = document_type.as_ref();
                let Some(referring) = declaring_document_type
                    .flattened_properties()
                    .get(referring_property)
                else {
                    return Ok(invalid(
                        "the declaring document type does not define the referring property",
                    ));
                };
                let Some(referenced) = referenced_document_type
                    .flattened_properties()
                    .get(referenced_property)
                else {
                    return Ok(invalid(
                        "the referenced document type does not define the referenced property",
                    ));
                };
                if matches!(referring.property_type, DocumentPropertyType::Object(_))
                    || matches!(referenced.property_type, DocumentPropertyType::Object(_))
                {
                    return Ok(invalid(
                        "agreement properties must be plain values, not object containers",
                    ));
                }
                if !same_value_kind(&referring.property_type, &referenced.property_type) {
                    return Ok(invalid(
                        "the two properties must share one value kind: a cross-kind \
                         equality could never be satisfied",
                    ));
                }
            }
        }
    }

    Ok(SimpleConsensusValidationResult::new())
}
