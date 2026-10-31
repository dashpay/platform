use versioned_feature_core::{FeatureVersion, FeatureVersionBounds, OptionalFeatureVersion};
pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;
pub mod v6;
pub mod v7;

#[derive(Clone, Debug, Default)]
pub struct DPPContractVersions {
    /// The maximum that we can store a data contract in the state. There is a possibility that a client
    /// sends a state transition serialized in a specific version and that the system re-serializes it
    /// to the current version, and in so doing increases it's size.
    pub max_serialized_size: u32,
    /// This is how we serialize and deserialize a contract
    pub contract_serialization_version: FeatureVersionBounds,
    /// The format a node stores a contract in: the `DataContractInSerializationFormat` variant
    /// Drive writes to state. It is a separate selector because a stored format need not be one
    /// that state transitions carry or clients are sent. Each protocol version so far selects the
    /// format of `contract_serialization_version`'s default; format 2, the stored format with
    /// compiled document types, is written only from the protocol version whose table selects it.
    pub contract_storage_serialization_version: FeatureVersion,
    /// The generation of a document type's stored form (`DocumentTypeInSerializationFormat`)
    /// that a contract stored in format 2 holds each of its document types in. The stored form
    /// carries its generation, so changing this changes only what is written from then on; a
    /// generation is paired with the builders whose output it stores, so one that rebuilds an
    /// index structure on load (generation 0 rebuilds it with `index_levels_from_indices` 0)
    /// is selected only with them.
    pub contract_storage_document_type_version: FeatureVersion,
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
    /// Accepted `TokenConfiguration` format versions. `max_version` 0 admits only `V0`; 1 also
    /// admits `V1`, which adds the per-token shielded pool flag. Contract creates and updates
    /// carrying a format above the bound are rejected with `UnsupportedVersionError`.
    pub token_configuration_format: FeatureVersionBounds,
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
    /// `fill_generated_properties`: writes each `generatedFrom` property a
    /// created or replaced document leaves out, generated from its params,
    /// before anything reads the document's data. `None` on versions that predate the
    /// keyword: the method leaves the data untouched there, so the shipped action
    /// transformers and proof verification that call it are inert.
    pub fill_generated_properties: OptionalFeatureVersion,
    /// `validate_generated_from_properties`: refuses a document whose
    /// `generatedFrom` property is not what its function generates from its
    /// params, or is present while a param is absent. `None` on versions that predate the
    /// keyword: the method returns an empty result there, so the shipped
    /// `DataContract::validate_document_properties` 0 that calls it is inert.
    pub validate_generated_from: OptionalFeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentTypeSchemaVersions {
    /// The contract grammar (document meta-schema generation). From generation 3 an index
    /// can key the elements of a typed array and time and integer windows.
    ///
    /// The SDK reads it too, to know from which protocol version every node serves
    /// `contains` / `containsAny` clauses and window selections
    /// (`DocumentQuery::required_protocol_version` in `dash-platform-queries`).
    pub document_type_schema: FeatureVersion,
    /// How full validation checks a document type's schema at registration. 0: against the
    /// document meta-schema `document_type_schema` names, through the `jsonschema` crate,
    /// which also compiles the schema. 1 (protocol version 15): with the rules of document
    /// meta-schema v3 in Platform's own code, an index property name held to 255 bytes, and
    /// the type required to have a typed validation schema, which its documents are checked
    /// against natively (`validate_document` 2).
    pub check_document_type_schema: FeatureVersion,
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
    /// entirely, exactly as they parsed before it existed. 0 matches the
    /// documents a `countOf` or `sumOf` totals by integer, string or
    /// identifier keys; 1 (protocol version 15) by byte array keys too.
    pub parse_property_constraints: OptionalFeatureVersion,
    /// Builds a document type's typed validation schema and its native validator
    /// (`document_type::schema::validation_schema`), which `validate_document` 2 checks
    /// documents against. `None` on versions whose document validation does not read one;
    /// 0 (protocol version 15) builds what the document validator of protocol version 14
    /// compiles.
    pub validation_schema: OptionalFeatureVersion,
    /// Parses the doctype-level `private` keyword (`{ "entrySize": n }`) onto the document
    /// type: a private document type, whose documents are fixed-size encrypted entries an
    /// anonymous shielded transition appends, with no index and no field the chain reads.
    /// On every parse it also holds the type to what a private type may declare, and under
    /// full validation `entrySize` to `max_private_document_entry_size`. Every version whose
    /// native schema check (`check_document_type_schema` 1) admits the key sets this
    /// (`should_parse_private_wherever_the_native_schema_check_admits_it`).
    /// `None` on versions that predate the keyword: they ignore it entirely, exactly as they
    /// parsed before it existed. Registration refuses it from protocol version 12 on
    /// (meta-schemas v1 to v3 close the doctype level), and no stored contract carries it: the
    /// protocol version 12 migration stripped the keys meta-schema v1 does not declare from
    /// the contracts admitted under meta-schema v0, which left the doctype level open. 0
    /// (protocol version 15).
    pub parse_private_document: OptionalFeatureVersion,
    /// Parses the `generatedFrom` property keyword (a string property whose
    /// value a built-in function generates from other properties of the same
    /// document) onto the property, and checks the properties it reads at
    /// contract registration. `None` on versions that predate the keyword: they ignore
    /// it entirely, exactly as they parsed before it existed.
    pub apply_generated_from: OptionalFeatureVersion,
    /// `DocumentType::expand_property_type_shorthands`: rewrites the property
    /// type shorthands of a document type schema and of the contract's
    /// `$defs` into the long form they stand for, `"type": "identifier"` into
    /// a 32-byte identifier byte array and `"type": "bytes"` with `"size": n`
    /// into a byte array of exactly `n` bytes, so the meta-schema, the parser,
    /// the document validator and the contract update comparison read one
    /// form. The contract is stored as sent. `None` on versions that predate
    /// the shorthands: the schema is returned as sent, and every earlier
    /// meta-schema refuses both.
    pub expand_property_type_shorthands: OptionalFeatureVersion,
    pub validate_max_depth: FeatureVersion,
    pub max_depth: u16,
    pub recursive_schema_validator_versions: RecursiveSchemaValidatorVersions,
    pub validate_schema_compatibility: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct RecursiveSchemaValidatorVersions {
    pub traversal_validator: FeatureVersion,
}

#[cfg(test)]
mod tests {
    use crate::version::protocol_version::PLATFORM_VERSIONS;

    /// The native schema check (`check_document_type_schema` 1) admits `private` as a key of a
    /// document schema, and only parser generation 3 under `parse_private_document` reads it:
    /// a table selecting the check without both would register a public type whose schema
    /// says it is private.
    #[test]
    fn should_parse_private_wherever_the_native_schema_check_admits_it() {
        let admitting: Vec<_> = PLATFORM_VERSIONS
            .iter()
            .filter(|platform_version| {
                platform_version
                    .dpp
                    .contract_versions
                    .document_type_versions
                    .schema
                    .check_document_type_schema
                    >= 1
            })
            .collect();
        assert!(
            !admitting.is_empty(),
            "expected a protocol version selecting the native schema check"
        );
        for platform_version in admitting {
            let document_type_versions = &platform_version
                .dpp
                .contract_versions
                .document_type_versions;
            assert!(
                document_type_versions
                    .schema
                    .parse_private_document
                    .is_some(),
                "protocol version {} admits `private` without parsing it",
                platform_version.protocol_version
            );
            assert!(
                document_type_versions.class_method_versions.try_from_schema >= 3,
                "protocol version {} admits `private` with a parser that does not read it",
                platform_version.protocol_version
            );
        }
    }
}
