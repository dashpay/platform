use versioned_feature_core::{FeatureVersion, FeatureVersionBounds, OptionalFeatureVersion};
pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;
pub mod v6;

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
    /// `ContractModerationConfig::validate` (protocol version 14); never reached before.
    pub validate_moderation_config: FeatureVersion,
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
    /// `TokenPreProgrammedDistribution::validate_amounts`. Called from protocol version 14 on
    /// (data contract create `basic_structure` v2 and `DataContract::validate_update` v1).
    pub validate_pre_programmed_distribution_amounts: FeatureVersion,
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
    /// `validate_distinct_from_properties`: refuses a document whose `distinctFrom`
    /// property equals the value it must differ from. `None` on versions that
    /// predate the keyword, where no parsed property carries it.
    pub validate_distinct_from: OptionalFeatureVersion,
    /// `validate_encrypted_property_shapes`: refuses a document whose `encryptedFor`
    /// property does not have the shape its scheme produces. `None` on versions
    /// that predate the keyword: the method returns an empty result there, so the
    /// shipped create and replace structure validations that call it are inert.
    pub validate_encrypted_property_shapes: OptionalFeatureVersion,
    /// `validate_max_bytes_properties`: refuses a document supplying a string longer in
    /// UTF-8 bytes than the `maxBytes` its property declares. `None` on versions that
    /// predate the keyword: the method returns an empty result there, so the shipped
    /// document validation that calls it is inert.
    pub validate_max_bytes: OptionalFeatureVersion,
    /// `validate_property_constraints`: refuses a document that breaks one of
    /// its type's `propertyConstraints`. `None` on versions that predate the
    /// keyword: the method returns an empty result there, so the shipped
    /// `DataContract::validate_document_properties` 0 that calls it is inert.
    pub validate_property_constraints: OptionalFeatureVersion,
    /// `Index::extract_contested_values`: writes an identifier property given as bytes or as
    /// an array of byte values as `Value::Identifier` in a contest's index values, so every
    /// contender names one contest with one poll. `None` on versions that predate it, where
    /// the values are taken as given.
    pub canonical_contested_index_values: OptionalFeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentTypeSchemaVersions {
    pub document_type_schema: FeatureVersion,
    pub should_add_creator_id: FeatureVersion,
    pub enrich_with_base_schema: FeatureVersion,
    pub find_identifier_and_binary_paths: FeatureVersion,
    /// Folds the `refersTo` reference keyword into the parsed property type.
    /// `None` on versions that predate the keyword: they ignore it entirely,
    /// exactly as they parsed before it existed.
    pub apply_property_reference: OptionalFeatureVersion,
    /// Parses the `requiredSince` property keyword (the contract version from
    /// which a property is required). `None` on versions that predate the
    /// keyword: they ignore it entirely, exactly as they parsed before it
    /// existed.
    pub apply_required_since: OptionalFeatureVersion,
    /// Parses the `distinctFrom` property keyword (an identifier property whose
    /// value must differ from a named property of the same document, or from
    /// the document's `$ownerId`) onto the property. `None` on versions that
    /// predate the keyword: they ignore it entirely, exactly as they parsed
    /// before it existed.
    pub apply_distinct_from: OptionalFeatureVersion,
    /// Parses the `encryptedFor` property keyword (how a byte array property's
    /// ciphertext was produced: recipient, key ids and scheme) onto the
    /// property, and checks the properties it names at contract registration.
    /// `None` on versions that predate the keyword: they ignore it entirely,
    /// exactly as they parsed before it existed.
    pub apply_encrypted_for: OptionalFeatureVersion,
    /// Folds the `maxBytes` keyword (the most UTF-8 bytes a string property, or
    /// each string element of a typed array, may hold) into the string's
    /// `StringPropertySizes`. `None` on versions that predate the keyword: they
    /// ignore it entirely, exactly as they parsed before it existed.
    pub apply_max_bytes: OptionalFeatureVersion,
    /// Parses a typed array property (`type: "array"` with an `items`
    /// element schema instead of `byteArray`). `None` on versions that
    /// predate typed arrays: they leave such a property to the scalar
    /// parser, which refuses an array that is not a byte array, exactly as
    /// they parsed before typed arrays existed.
    pub parse_typed_array: OptionalFeatureVersion,
    /// Parses the doctype-level `propertyConstraints` keyword (named
    /// comparisons between integer expressions over the document's
    /// properties) onto the document type, and checks the properties they
    /// read. `None` on versions that predate the keyword: they ignore it
    /// entirely, exactly as they parsed before it existed.
    pub parse_property_constraints: OptionalFeatureVersion,
    pub validate_max_depth: FeatureVersion,
    pub max_depth: u16,
    pub recursive_schema_validator_versions: RecursiveSchemaValidatorVersions,
    pub validate_schema_compatibility: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct RecursiveSchemaValidatorVersions {
    pub traversal_validator: FeatureVersion,
}
