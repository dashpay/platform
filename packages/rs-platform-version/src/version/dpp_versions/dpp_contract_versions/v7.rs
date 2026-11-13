use crate::version::dpp_versions::dpp_contract_versions::v6::CONTRACT_VERSIONS_V6;
use crate::version::dpp_versions::dpp_contract_versions::{
    DPPContractVersions, DataContractMethodVersions, DocumentTypeClassMethodVersions,
    DocumentTypeMethodVersions, DocumentTypeSchemaVersions, DocumentTypeVersions,
};

/// Introduced in protocol version 15. Differs from v6 in:
/// - `try_from_schema`, 4 rather than 3: parses `canBeErased` and admits deletable
///   keep-history types, while preserving generation 3's non-lifecycle parsing;
/// - `parse_property_constraints`, 1 rather than 0: a `countOf` or `sumOf` of a
///   `propertyConstraints` or `deleteConstraints` rule may match its documents by a byte array
///   key, bound to a byte array property of the document written (or deleted), as it matches
///   them by an integer, string or identifier key;
/// - `validate_document`, 2 rather than 1: a document is checked against its type's native
///   validator (the typed validation schema of `document_type::schema::validation_schema`)
///   instead of the `jsonschema` crate. A document type whose schema has no validation schema
///   has its documents refused;
/// - `check_document_type_schema`, 1 rather than 0: a schema is checked at registration
///   natively instead of against the document meta-schema through the `jsonschema` crate, and
///   every new document type must have a validation schema;
/// - `validate_max_depth`, 2 rather than 1: the depth and cycle check resolves `$ref`
///   pointers by RFC 6901, as the native validator does;
/// - `validation_schema`, `Some(0)` rather than `None`: builds that typed validation schema
///   and the native validator;
/// - `parse_private_document`, `Some(0)` rather than `None`: the `private` keyword declares a
///   private document type, which only the native schema check of registration admits;
/// - `apply_property_reference`, `Some(1)` rather than `Some(0)`: a permanent or moderated
///   document reference on an identifier property may name its target through the
///   referring document (`contractId` and `documentType` written `"$self.<property>"`), with what the named
///   document type must look like (`documentTypeRequirements`);
/// - `parse_contested_documents_resolution`, `Some(0)` rather than `None`: a contested index
///   may declare `"resolution": 2`, a masternode vote among documents, where each document
///   entered is a contender named by its document id and an identity may enter several;
/// - `apply_generated_from`, `Some(1)` rather than `Some(0)`: a `generatedFrom` property may
///   also be generated with `sys.tokens.decimals`, the decimals of the token an identifier
///   property names, on an integer property. Generation 1 parses `sys.random.uniform` too;
/// - `random_generated_from`, `Some(0)` rather than `None`: registration admits
///   `sys.random.uniform`, whose values block execution draws from the random beacon Tenderdash
///   delivers from consensus version 2 on (v15 notes 43 and 45);
/// - `validate_schema_patterns`, `Some(0)` rather than `None`: registration and update refuse
///   a contract whose document schemas and `$defs` hold more than
///   `max_contract_schema_patterns` patterns, or one that does not compile within
///   `max_schema_pattern_compiled_size`;
/// - `registration_cost`, 3 rather than 2: the fee a state transition's contract is estimated
///   at bills each index by the keys it declares instead of parsing it, which compiled the
///   field matches of a contested index before anything bounded them;
/// - `contract_storage_serialization_version`, 2 rather than 1: Drive stores a contract in
///   format version 2, its document types compiled, while state transitions and clients keep
///   format version 1;
/// - `canonical_contested_index_values`, `Some(1)` rather than `Some(0)`: every index value
///   of a contest is written the one way its property's type keys it, so the values of one
///   contest name one poll; 0 wrote only identifiers one way.
///
/// v6 remains unchanged for `PROTOCOL_VERSION_14`, whose parse refuses a byte array key and
/// whose schemas and documents the `jsonschema` crate checks.
pub const CONTRACT_VERSIONS_V7: DPPContractVersions = DPPContractVersions {
    methods: DataContractMethodVersions {
        // changed: the document type's native validator checks the schema in place of the
        // `jsonschema` crate: uniqueItems treats signed zeros as one value, and a
        // format:regex value is checked for syntax rather than compilation
        validate_document: 2,
        // changed: a contract's schemas hold a bounded number of patterns of bounded size
        validate_schema_patterns: Some(0),
        // changed: the fee estimate bills an index without compiling its field matches
        registration_cost: 3,
        ..CONTRACT_VERSIONS_V6.methods
    },
    document_type_versions: DocumentTypeVersions {
        class_method_versions: DocumentTypeClassMethodVersions {
            try_from_schema: 4, // lifecycle flags extend the schema-v3 grammar (note 41)
            ..CONTRACT_VERSIONS_V6
                .document_type_versions
                .class_method_versions
        },
        schema: DocumentTypeSchemaVersions {
            // changed: checks a schema natively at registration and requires a validation schema
            check_document_type_schema: 1,
            parse_property_constraints: Some(1), // changed: countOf and sumOf match byte array keys
            // changed: documents are checked against the typed validation schema this builds
            validation_schema: Some(0),
            // changed: the `private` keyword declares a private document type
            parse_private_document: Some(0),
            // changed: a contested index may make each document a contender (`resolution` 2)
            parse_contested_documents_resolution: Some(0),
            apply_generated_from: Some(1), // changed: admits sys.tokens.decimals
            // changed: also reads a reference whose target the referring document names
            apply_property_reference: Some(1),
            // changed: sys.random.uniform is admitted, block execution drawing its values from
            // each block's random beacon
            random_generated_from: Some(0),
            // changed: the depth and cycle check reads a `$ref` pointer as the validator does
            // and holds arrays to the maximum depth as it does maps
            validate_max_depth: 2,
            ..CONTRACT_VERSIONS_V6.document_type_versions.schema
        },
        methods: DocumentTypeMethodVersions {
            // changed: every index value of a contest is written the one way its property's
            // type keys it, where 0 wrote only identifiers one way
            canonical_contested_index_values: Some(1),
            ..CONTRACT_VERSIONS_V6.document_type_versions.methods
        },
        ..CONTRACT_VERSIONS_V6.document_type_versions
    },
    // changed: contracts are stored with their document types compiled, about 1.8 times the
    // bytes in state and in proofs (v15 note 8)
    contract_storage_serialization_version: 2,
    ..CONTRACT_VERSIONS_V6
};
