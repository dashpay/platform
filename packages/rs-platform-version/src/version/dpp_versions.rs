use crate::version::{FeatureVersion, FeatureVersionBounds};


#[derive(Clone, Debug, Default)]
pub struct DPPVersion {
    pub costs: CostVersions,
    pub validation: DPPValidationVersions,
    pub state_transition_serialization_versions: StateTransitionSerializationVersions,
    pub state_transition_conversion_versions: StateTransitionConversionVersions,
    pub state_transition_method_versions: StateTransitionMethodVersions,
    pub contract_versions: ContractVersions,
    pub document_versions: DocumentVersions,
    pub identity_versions: IdentityVersions,
}

#[derive(Clone, Debug, Default)]
pub struct CostVersions {
    pub signature_verify: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DPPValidationVersions {
    pub validate_time_in_block_time_window: FeatureVersion,
    pub json_schema_validator: JsonSchemaValidatorVersions,
    pub data_contract: DataContractValidationVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DataContractValidationVersions {
    pub validate: FeatureVersion,
    pub validate_index_definitions: FeatureVersion,
    pub validate_index_naming_duplicates: FeatureVersion,
    pub validate_not_defined_properties: FeatureVersion,
    pub validate_property_definition: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct JsonSchemaValidatorVersions {
    pub get_schema_compilation_options: FeatureVersion,
    pub new: FeatureVersion,
    pub new_with_definitions: FeatureVersion,
    pub validate: FeatureVersion,
    pub validate_data_contract_schema: FeatureVersion,
    pub validate_schema: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct StateTransitionMethodVersions {
    pub public_key_in_creation_methods: PublicKeyInCreationMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct PublicKeyInCreationMethodVersions {
    pub from_public_key_signed_with_private_key: FeatureVersion,
    pub from_public_key_signed_external: FeatureVersion,
    pub hash: FeatureVersion,
    pub duplicated_key_ids_witness: FeatureVersion,
    pub duplicated_keys_witness: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct StateTransitionConversionVersions {
    pub identity_to_identity_create_transition: FeatureVersion,
    pub identity_to_identity_create_transition_with_signer: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct StateTransitionSerializationVersions {
    pub identity_create_state_transition: FeatureVersionBounds,
    pub identity_update_state_transition: FeatureVersionBounds,
    pub identity_top_up_state_transition: FeatureVersionBounds,
    pub identity_credit_withdrawal_state_transition: FeatureVersionBounds,
    pub identity_credit_transfer_state_transition: FeatureVersionBounds,
    pub contract_create_state_transition: FeatureVersionBounds,
    pub contract_update_state_transition: FeatureVersionBounds,
    pub documents_batch_state_transition: FeatureVersionBounds,
    pub document_base_state_transition: FeatureVersionBounds,
    pub document_create_state_transition: DocumentFeatureVersionBounds,
    pub document_replace_state_transition: DocumentFeatureVersionBounds,
    pub document_delete_state_transition: DocumentFeatureVersionBounds,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentFeatureVersionBounds {
    pub bounds: FeatureVersionBounds,
}

#[derive(Clone, Debug, Default)]
pub struct ContractVersions {
    /// This is how we serialize and deserialize a contract
    pub contract_serialization_version: FeatureVersionBounds,
    /// This is the structure of the Contract as it is defined for code paths
    pub contract_structure_version: FeatureVersion,
    pub created_data_contract_structure_version: FeatureVersion,
    pub config_version: FeatureVersion,
    pub document_type_versions: DocumentTypeVersions,
    pub index_versions: IndexVersions,
    pub contract_class_method_versions: ContractClassMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct ContractClassMethodVersions {
    pub get_property_definition_by_path: FeatureVersion,
    pub get_binary_properties_from_schema: FeatureVersion,
    pub get_definitions: FeatureVersion,
    pub get_document_types_from_contract: FeatureVersion,
    pub get_document_types_from_value: FeatureVersion,
    pub get_document_types_from_value_array: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct IndexVersions {
    pub index_levels_from_indices: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentTypeVersions {
    /// This is for the overall structure of the document type, like DocumentTypeV0
    pub document_type_structure_version: FeatureVersion,
    /// Class based method
    pub find_identifier_and_binary_paths: FeatureVersion,
    /// Class based method
    pub insert_values: FeatureVersion,
    /// Class based method
    pub insert_values_nested: FeatureVersion,
    pub index_for_types: FeatureVersion,
    pub unique_id_for_storage: FeatureVersion,
    pub unique_id_for_document_field: FeatureVersion,
    pub serialize_value_for_key: FeatureVersion,
    pub convert_value_to_document: FeatureVersion,
    pub create_document_from_data: FeatureVersion,
    pub max_size: FeatureVersion,
    pub estimated_size: FeatureVersion,
    pub create_document_with_prevalidated_properties: FeatureVersion,
    pub top_level_indices: FeatureVersion,
    pub document_field_for_property: FeatureVersion,
    pub document_field_type_for_property: FeatureVersion,
    pub field_can_be_null: FeatureVersion,
    pub initial_revision: FeatureVersion,
    pub requires_revision: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct IdentityVersions {
    /// This is the structure of the Identity as it is defined for code paths
    pub identity_structure_version: FeatureVersion,
    pub identity_key_structure_version: FeatureVersion,
    pub identity_key_type_method_versions: IdentityKeyTypeMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct IdentityKeyTypeMethodVersions {
    pub random_public_key_data: FeatureVersion,
    pub random_public_and_private_key_data: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentVersions {
    // This is for the overall structure of the document, like DocumentV0
    pub document_structure_version: FeatureVersion,
    pub document_serialization_version: FeatureVersionBounds,
    pub extended_document_structure_version: FeatureVersionBounds,
    pub document_method_versions: DocumentMethodVersions,
    pub document_class_method_versions: DocumentClassMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentClassMethodVersions {
    pub get_identifiers_and_binary_paths: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentMethodVersions {
    pub hash: FeatureVersion,
    pub get_raw_for_contract: FeatureVersion,
    pub get_raw_for_document_type: FeatureVersion,
}
