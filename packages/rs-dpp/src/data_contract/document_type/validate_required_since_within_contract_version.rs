use std::collections::BTreeMap;

use crate::consensus::basic::data_contract::DataContractInvalidRequiredFieldsUpdateError;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentType;

/// A `requiredSince` annotation may never exceed the version of the contract
/// carrying it — requiredness cannot be pre-scheduled at a future version.
/// Runs over the *parsed* properties, so annotations reached through `$ref`
/// are covered. Called wherever document types are built from a contract's
/// serialized form (creates, updates, and disk loads all pass through
/// there); a no-op for every contract predating the keyword, since their
/// properties carry no annotation.
///
/// The failure is the dedicated consensus error, because the input is
/// untrusted schema data: state-transition processing must classify it as
/// consensus-invalid (nonce bump), never as an execution error. Callers map
/// it through
/// [`class_methods::consensus_or_protocol_required_fields_error`](crate::data_contract::document_type::class_methods).
pub(crate) fn validate_required_since_within_contract_version(
    document_types: &BTreeMap<String, DocumentType>,
    contract_version: u32,
) -> Result<(), DataContractInvalidRequiredFieldsUpdateError> {
    for (document_type_name, document_type) in document_types {
        for (property_name, property) in document_type.as_ref().properties() {
            if let Some(required_since) = property.required_since {
                if required_since > contract_version {
                    return Err(DataContractInvalidRequiredFieldsUpdateError::new(
                        document_type_name.clone(),
                        format!(
                            "property '{property_name}' carries requiredSince {required_since} which exceeds the contract version {contract_version}"
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}
