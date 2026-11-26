use crate::version::dpp_versions::dpp_contract_versions::v7::CONTRACT_VERSIONS_V7;
use crate::version::dpp_versions::dpp_contract_versions::{
    DPPContractVersions, DocumentTypeSchemaVersions, DocumentTypeVersions,
};

/// Introduced in protocol version 16. Differs from v7 in:
/// - `random_generated_from`, `Some(0)` rather than `None`: registration admits
///   `sys.random.uniform`, which `apply_generated_from` 1 parses at protocol version 15 too,
///   since block execution draws its values from the random beacon Tenderdash delivers from
///   consensus version 3 on (v16 notes 2 and 4).
pub const CONTRACT_VERSIONS_V8: DPPContractVersions = DPPContractVersions {
    document_type_versions: DocumentTypeVersions {
        schema: DocumentTypeSchemaVersions {
            random_generated_from: Some(0), // changed: sys.random.uniform is admitted
            ..CONTRACT_VERSIONS_V7.document_type_versions.schema
        },
        ..CONTRACT_VERSIONS_V7.document_type_versions
    },
    ..CONTRACT_VERSIONS_V7
};
