use crate::fee::Credits;
use platform_version::version::PlatformVersion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolValidationOperation {
    DocumentTypeSchemaValidationForSize(u64),
    DocumentTypeSchemaPropertyValidation(u64),
    DocumentTypeSchemaIndexValidation(u64, bool),
    /// Compiling and serializing a contract to check its compiled size, of that many bytes
    /// (protocol version 15): charged per byte at the rate of a document type schema's size.
    ContractCompiledSizeValidation(u64),
}

impl ProtocolValidationOperation {
    pub fn processing_cost(&self, platform_version: &PlatformVersion) -> Credits {
        match self {
            ProtocolValidationOperation::DocumentTypeSchemaValidationForSize(size) => {
                platform_version
                    .fee_version
                    .data_contract_validation
                    .document_type_base_fee
                    + *size
                        * platform_version
                            .fee_version
                            .data_contract_validation
                            .document_type_schema_size_fee
            }
            ProtocolValidationOperation::ContractCompiledSizeValidation(compiled_size) => {
                *compiled_size
                    * platform_version
                        .fee_version
                        .data_contract_validation
                        .document_type_schema_size_fee
            }
            ProtocolValidationOperation::DocumentTypeSchemaPropertyValidation(properties_count) => {
                *properties_count
                    * platform_version
                        .fee_version
                        .data_contract_validation
                        .document_type_per_property_fee
            }
            ProtocolValidationOperation::DocumentTypeSchemaIndexValidation(
                index_properties_count,
                is_unique,
            ) => {
                if *is_unique {
                    *index_properties_count
                        * platform_version
                            .fee_version
                            .data_contract_validation
                            .document_type_unique_index_per_property_fee
                        + platform_version
                            .fee_version
                            .data_contract_validation
                            .document_type_base_unique_index_fee
                } else {
                    *index_properties_count
                        * platform_version
                            .fee_version
                            .data_contract_validation
                            .document_type_non_unique_index_per_property_fee
                        + platform_version
                            .fee_version
                            .data_contract_validation
                            .document_type_base_non_unique_index_fee
                }
            }
        }
    }
}
