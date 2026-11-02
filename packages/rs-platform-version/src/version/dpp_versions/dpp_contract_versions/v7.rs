use crate::version::dpp_versions::dpp_contract_versions::v6::CONTRACT_VERSIONS_V6;
use crate::version::dpp_versions::dpp_contract_versions::{
    DPPContractVersions, DataContractMethodVersions, DocumentTypeSchemaVersions,
    DocumentTypeVersions,
};

/// Introduced in protocol version 15. Differs from v6 in:
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
/// - `validation_schema`, `Some(0)` rather than `None`: builds that typed validation schema
///   and the native validator;
/// - `parse_private_document`, `Some(0)` rather than `None`: the `private` keyword declares a
///   private document type, which only the native schema check of registration admits;
/// - `apply_generated_from`, `Some(1)` rather than `Some(0)`: a `generatedFrom` property may
///   also be generated with `sys.tokens.decimals`, the decimals of the token an identifier
///   property names, on an integer property;
/// - `contract_storage_serialization_version`, 2 rather than 1: Drive stores a contract in
///   format version 2, its document types compiled, while state transitions and clients keep
///   format version 1.
///
/// v6 remains unchanged for `PROTOCOL_VERSION_14`, whose parse refuses a byte array key and
/// whose schemas and documents the `jsonschema` crate checks.
pub const CONTRACT_VERSIONS_V7: DPPContractVersions = DPPContractVersions {
    methods: DataContractMethodVersions {
        // changed: the document type's native validator checks the schema in place of the
        // `jsonschema` crate, reaching the same verdicts but for one rule of its own: in a
        // `uniqueItems` list, `0.0` and `-0.0` are one value however long the list
        validate_document: 2,
        ..CONTRACT_VERSIONS_V6.methods
    },
    document_type_versions: DocumentTypeVersions {
        schema: DocumentTypeSchemaVersions {
            // changed: checks a schema natively at registration and requires a validation schema
            check_document_type_schema: 1,
            parse_property_constraints: Some(1), // changed: countOf and sumOf match byte array keys
            // changed: documents are checked against the typed validation schema this builds
            validation_schema: Some(0),
            // changed: the `private` keyword declares a private document type
            parse_private_document: Some(0),
            apply_generated_from: Some(1), // changed: admits sys.tokens.decimals
            ..CONTRACT_VERSIONS_V6.document_type_versions.schema
        },
        ..CONTRACT_VERSIONS_V6.document_type_versions
    },
    // changed: contracts are stored with their document types compiled, about 1.8 times the
    // bytes in state and in proofs (v15 note 8)
    contract_storage_serialization_version: 2,
    ..CONTRACT_VERSIONS_V6
};
