use versioned_feature_core::{FeatureVersion, FeatureVersionBounds};
pub mod v1;
pub mod v2;

#[derive(Clone, Debug, Default)]
pub struct DPPContractVersions {
    /// The maximum that we can store a data contract in the state. There is a possibility that a client
    /// sends a state transition serialized in a specific version and that the system re-serializes it
    /// to the current version, and in so doing increases it's size.
    pub max_serialized_size: u32,
    /// This is how we serialize and deserialize a contract
    pub contract_serialization_version: FeatureVersionBounds,
    /// This is the structure of the Contract as it is defined for code paths
    pub contract_structure_version: FeatureVersion,
    pub created_data_contract_structure: FeatureVersion,
    pub config: FeatureVersionBounds,
    pub methods: DataContractMethodVersions,
    pub document_type_versions: DocumentTypeVersions,
    pub token_versions: TokenVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DataContractMethodVersions {
    pub validate_document: FeatureVersion,
    pub validate_update: FeatureVersion,
    pub schema: FeatureVersion,
    pub validate_groups: FeatureVersion,
    pub equal_ignoring_time_fields: FeatureVersion,
    pub registration_cost: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentTypeClassMethodVersions {
    pub try_from_schema: FeatureVersion,
    pub create_document_types_from_document_schemas: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentTypeIndexVersions {
    pub index_levels_from_indices: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentTypeVersions {
    pub index_versions: DocumentTypeIndexVersions,
    pub class_method_versions: DocumentTypeClassMethodVersions,
    /// This is for the overall structure of the document type, like DocumentTypeV0
    pub structure_version: FeatureVersion,
    pub schema: DocumentTypeSchemaVersions,
    pub methods: DocumentTypeMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct TokenVersions {
    pub validate_structure_interval: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentTypeMethodVersions {
    pub create_document_from_data: FeatureVersion,
    pub create_document_with_prevalidated_properties: FeatureVersion,
    pub prefunded_voting_balance_for_document: FeatureVersion,
    pub contested_vote_poll_for_document: FeatureVersion,
    pub estimated_size: FeatureVersion,
    pub index_for_types: FeatureVersion,
    pub max_size: FeatureVersion,
    pub serialize_value_for_key: FeatureVersion,
    pub deserialize_value_for_key: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentTypeSchemaVersions {
    pub schema_version: FeatureVersion,
    pub enrich_with_base_schema: FeatureVersion,
    pub find_identifier_and_binary_paths: FeatureVersion,
    pub validate_max_depth: FeatureVersion,
    pub max_depth: u16,
    pub recursive_schema_validator_versions: RecursiveSchemaValidatorVersions,
    pub validate_schema_compatibility: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct RecursiveSchemaValidatorVersions {
    pub traversal_validator: FeatureVersion,
}
