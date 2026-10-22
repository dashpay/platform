use crate::version::dpp_versions::dpp_contract_versions::v6::CONTRACT_VERSIONS_V6;
use crate::version::dpp_versions::dpp_contract_versions::{
    DPPContractVersions, DataContractMethodVersions, DocumentTypeClassMethodVersions,
    DocumentTypeSchemaVersions, DocumentTypeVersions,
};

/// Introduced in protocol version 15. Differs from v6 in:
/// - `parse_property_constraints`, 1 rather than 0: a `countOf` or `sumOf` of a
///   `propertyConstraints` or `deleteConstraints` rule may match its documents by a byte array
///   key, bound to a byte array property of the document written (or deleted), as it matches
///   them by an integer, string or identifier key;
/// - `validate_document`, 2 rather than 1: a document is checked against its type's native
///   validator (the typed validation schema of `document_type::schema::validation_schema`)
///   instead of the `jsonschema` crate. A document type whose schema has no validation schema
///   keeps generation 1's check;
/// - `try_from_schema`, 4 rather than 3: parser generation 4 checks a schema at registration
///   natively instead of against the document meta-schema through the `jsonschema` crate, and
///   requires every new document type to have a validation schema.
///
/// v6 remains unchanged for `PROTOCOL_VERSION_14`, whose parse refuses a byte array key and
/// whose schemas and documents the `jsonschema` crate checks.
pub const CONTRACT_VERSIONS_V7: DPPContractVersions = DPPContractVersions {
    methods: DataContractMethodVersions {
        // changed: the document type's native validator checks the schema in place of the
        // `jsonschema` crate, reaching the same verdicts
        validate_document: 2,
        ..CONTRACT_VERSIONS_V6.methods
    },
    document_type_versions: DocumentTypeVersions {
        class_method_versions: DocumentTypeClassMethodVersions {
            // changed: parser generation 4, generation 3 with native schema checks in place
            // of the document meta-schema
            try_from_schema: 4,
            ..CONTRACT_VERSIONS_V6
                .document_type_versions
                .class_method_versions
        },
        schema: DocumentTypeSchemaVersions {
            parse_property_constraints: Some(1), // changed: countOf and sumOf match byte array keys
            ..CONTRACT_VERSIONS_V6.document_type_versions.schema
        },
        ..CONTRACT_VERSIONS_V6.document_type_versions
    },
    ..CONTRACT_VERSIONS_V6
};
