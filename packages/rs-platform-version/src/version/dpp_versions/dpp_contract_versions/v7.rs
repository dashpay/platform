use crate::version::dpp_versions::dpp_contract_versions::{
    DPPContractVersions, DataContractMethodVersions, DocumentTypeClassMethodVersions,
    DocumentTypeIndexVersions, DocumentTypeMethodVersions, DocumentTypeSchemaVersions,
    DocumentTypeVersions, RecursiveSchemaValidatorVersions, TokenVersions,
};
use versioned_feature_core::FeatureVersionBounds;

// Introduced in protocol version 15: `CONTRACT_VERSIONS_V6` with `validate_document` 2,
// whose generation checks a document against its type's native validator (the typed
// validation schema of `document_type::schema::validation_schema`) instead of the
// `jsonschema` crate. A document type whose schema has no validation schema keeps
// generation 1's check. Every other entry is V6's.
pub const CONTRACT_VERSIONS_V7: DPPContractVersions = DPPContractVersions {
    max_serialized_size: 65000,
    contract_serialization_version: FeatureVersionBounds {
        min_version: 0,
        max_version: 1,
        default_current_version: 1,
    },
    contract_structure_version: 1,
    created_data_contract_structure: 0,
    // Config V2 (protocol version 14) carries the optional contract moderation declaration.
    // The default is 2 too: from this protocol version every new contract carries a V2
    // config, moderated or not. The config version follows the platform version.
    config: FeatureVersionBounds {
        min_version: 1,
        max_version: 2,
        default_current_version: 2,
    },
    methods: DataContractMethodVersions {
        // changed: the document type's native validator checks the schema in place of the
        // `jsonschema` crate, reaching the same verdicts
        validate_document: 2,
        // Generation 1 (requiredSince): feeds the new contract version into
        // per-document-type update validation and validates requiredSince
        // annotations on document types introduced by the update, which the
        // per-type pass never sees. Generation 0 stays byte-identical for
        // replay of pre-v14 blocks.
        validate_update: 1,
        schema: 0,
        validate_groups: 0,
        validate_moderation_config: 0,
        equal_ignoring_time_fields: 0,
        registration_cost: 2, // changed: charges the once-per-identity token distribution surcharge
    },
    document_type_versions: DocumentTypeVersions {
        index_versions: DocumentTypeIndexVersions {
            index_levels_from_indices: 0,
        },
        class_method_versions: DocumentTypeClassMethodVersions {
            try_from_schema: 3, // changed: parser generation 3 — generation 2 plus the ranked index keywords
            create_document_types_from_document_schemas: 1,
        },
        structure_version: 0,
        schema: DocumentTypeSchemaVersions {
            document_type_schema: 3, // changed: v3 document meta-schema — v2 plus the ranked index keywords, and the gate the index parser reads
            should_add_creator_id: 1,
            enrich_with_base_schema: 1,
            find_identifier_and_binary_paths: 1, // changed: a typed array's identifier and byte array elements are registered as `path[]` conversion paths
            apply_property_reference: Some(0), // changed: the meta-schema v3 `refersTo` keyword is folded into the parsed property type; None before this version means the keyword is ignored, as it was before it existed
            apply_required_since: Some(0), // changed: the meta-schema v3 `requiredSince` keyword (contract version a property is required from) is parsed onto the property; None before this version means the keyword is ignored, as it was before it existed
            apply_distinct_from: Some(0), // changed: the meta-schema v3 `distinctFrom` keyword (an identifier property whose value must differ from a named sibling property or the document's `$ownerId`) is parsed onto the property; None before this version means the keyword is ignored, as it was before it existed
            apply_encrypted_for: Some(0), // changed: the meta-schema v3 `encryptedFor` keyword (recipient, key ids and scheme of a byte array property's ciphertext) is parsed onto the property and its named properties are checked at registration; None before this version means the keyword is ignored, as it was before it existed
            apply_max_bytes: Some(0), // changed: the meta-schema v3 `maxBytes` keyword (the most UTF-8 bytes a string property, or each string element of a typed array, may hold) is folded into the string's `StringPropertySizes`; None before this version means the keyword is ignored, as it was before it existed
            parse_typed_array: Some(0), // changed: a meta-schema v3 typed array (`type: "array"` with an `items` element schema) parses to `DocumentPropertyType::TypedArray`; None before this version leaves it to the scalar parser, which refuses an array that is not a byte array
            parse_property_constraints: Some(0), // changed: the meta-schema v3 `propertyConstraints` doctype keyword (named comparisons between integer expressions over the document's properties) is parsed onto the document type and the properties it reads are checked; None before this version means the keyword is ignored, as it was before it existed
            apply_generated_from: Some(0), // changed: the meta-schema v3 `generatedFrom` keyword (a string property whose value a built-in function generates from other properties of the same document) is parsed onto the property and the properties it reads are checked at registration; None before this version means the keyword is ignored, as it was before it existed
            expand_property_type_shorthands: Some(0), // new: a property schema of `"type": "identifier"`, or `"type": "bytes"` with a `size`, is read as the byte array it stands for, before the meta-schema, the parser and the document validator see it; the contract is stored as sent. None before this version leaves the schema as sent, which every earlier meta-schema refuses
            validate_max_depth: 1, // changed: a scalar `$ref` target is not walked and the visited set is never cleared, so the depth check is bounded; v0 was exponential on a crafted `$defs` chain
            max_depth: 256,
            recursive_schema_validator_versions: RecursiveSchemaValidatorVersions {
                traversal_validator: 0,
            },
            validate_schema_compatibility: 1, // changed: strips `indices` before diffing — index changes are validated by `validate_update` v1, so an index-order-only diff no longer hard-errors
        },
        methods: DocumentTypeMethodVersions {
            create_document_from_data: 0,
            create_document_with_prevalidated_properties: 0,
            prefunded_voting_balance_for_document: 0,
            contested_vote_poll_for_document: 0,
            estimated_size: 1, // changed: adds the document serialization format 3 contract-version stamp varint (worst case 5 bytes) to the estimate
            // Changed: v1 requires a query's bound fields to cover a
            // contiguous prefix of the candidate index (equalities exactly
            // covering the leading properties, range/in immediately after,
            // no unused property before an order-by field). v0 matched by
            // positionless set membership, so a query binding only later
            // index properties selected an index the positional path
            // lowering misaligns on — returning cryptographically proven
            // wrong or empty results. Gapped candidates are now skipped
            // per-candidate, letting a well-shaped index win or the query
            // fail with WhereClauseOnNonIndexedProperty. v0 stays frozen
            // for replay at protocol versions <= 13.
            index_for_types: 1,
            max_size: 0,
            serialize_value_for_key: 1, // changed: a property (or derived index property) of an unsigned integer type is keyed by its plain big-endian bytes, so its keys sort as its values do; generation 0 flipped the top bit as for a signed value. The `$` system fields, and an unsigned property ending a `rangeSummable` index, keep their keys. The key of a typed array property is one of its elements, keyed as a property of the element's type
            deserialize_value_for_key: 1, // changed: reads the generation 1 keys, a typed array key decoding to one element
            validate_distinct_from: Some(0), // changed: `validate_distinct_from_properties` refuses a document whose `distinctFrom` property equals what it must differ from (DocumentPropertyNotDistinctError, 10419); None before this version, where no property can carry the keyword
            validate_encrypted_property_shapes: Some(0), // changed: refuses an `encryptedFor` property whose bytes are not the shape its scheme produces; None before this version returns an empty result
            validate_max_bytes: Some(0), // changed: refuses a string longer in UTF-8 bytes than its property's `maxBytes` (DocumentPropertyMaxBytesExceededError, 10421); None before this version returns an empty result
            validate_property_constraints: Some(0), // changed: `validate_property_constraints` refuses a created or replaced document that breaks one of its type's `propertyConstraints` (DocumentPropertyConstraintViolatedError, 10422); None before this version returns an empty result
            canonical_contested_index_values: Some(0), // new: identifier index values of a contest are written as identifiers
            fill_generated_properties: Some(0), // new: a created or replaced document that leaves out a `generatedFrom` property whose params it supplies gets the property generated on arrival; None before this version leaves the data as sent
            validate_generated_from: Some(0), // new: `validate_generated_from_properties` refuses a document whose `generatedFrom` property is not what its function generates from its params, or is present without them (DocumentPropertyNotGeneratedError, 10424); None before this version returns an empty result
        },
    },
    token_versions: TokenVersions {
        // 1: an epoch-based perpetual distribution needs an interval of at least one epoch.
        validate_structure_interval: 1,
        validate_pre_programmed_distribution_amounts: 0,
        token_configuration_format: FeatureVersionBounds {
            min_version: 0,
            max_version: 1,
            default_current_version: 0,
        },
    },
};
