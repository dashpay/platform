use crate::version::dpp_versions::dpp_contract_versions::v6::CONTRACT_VERSIONS_V6;
use crate::version::dpp_versions::dpp_contract_versions::{
    DPPContractVersions, DocumentTypeSchemaVersions, DocumentTypeVersions,
};

/// Introduced in protocol version 15. Differs from v6 in `parse_property_constraints`, 1 rather
/// than 0: a `countOf` or `sumOf` of a `propertyConstraints` or `deleteConstraints` rule may
/// match its documents by a byte array key, bound to a byte array property of the document
/// written (or deleted), as it matches them by an integer, string or identifier key. v6 remains
/// unchanged for `PROTOCOL_VERSION_14`, whose parse refuses such a key.
pub const CONTRACT_VERSIONS_V7: DPPContractVersions = DPPContractVersions {
    document_type_versions: DocumentTypeVersions {
        schema: DocumentTypeSchemaVersions {
            parse_property_constraints: Some(1), // changed: countOf and sumOf match byte array keys
            ..CONTRACT_VERSIONS_V6.document_type_versions.schema
        },
        ..CONTRACT_VERSIONS_V6.document_type_versions
    },
    ..CONTRACT_VERSIONS_V6
};
