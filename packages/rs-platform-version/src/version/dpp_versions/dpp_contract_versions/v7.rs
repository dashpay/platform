use crate::version::dpp_versions::dpp_contract_versions::v6::CONTRACT_VERSIONS_V6;
use crate::version::dpp_versions::dpp_contract_versions::{
    DPPContractVersions, DocumentTypeClassMethodVersions, DocumentTypeSchemaVersions,
    DocumentTypeVersions,
};

// Introduced in protocol version 15 for the delete and erase lifecycle of
// keep-history documents. Uses the v4 document meta-schema, which is v3 plus
// the `canBeErased` keyword.
//
// `try_from_schema` moves to 4, selecting a new document-type parser
// generation (`try_from_schema/v4`). Generation 4 admits `documentsKeepHistory`
// together with `canBeDeleted` (a delete leaves the retained revisions
// readable), parses `canBeErased` (a deleted document's revisions may be
// purged), and refuses a keep-history type that carries a contested index.
// Generation 3 keeps rejecting deletable keep-history types, so replaying a
// protocol 14 block validates contracts exactly as it did.
//
// `document_type_schema` moves to 4 in the same step: generation 4 and
// meta-schema v4 are introduced together and pair by construction.
pub const CONTRACT_VERSIONS_V7: DPPContractVersions = DPPContractVersions {
    document_type_versions: DocumentTypeVersions {
        class_method_versions: DocumentTypeClassMethodVersions {
            try_from_schema: 4,
            ..CONTRACT_VERSIONS_V6
                .document_type_versions
                .class_method_versions
        },
        schema: DocumentTypeSchemaVersions {
            document_type_schema: 4,
            ..CONTRACT_VERSIONS_V6.document_type_versions.schema
        },
        ..CONTRACT_VERSIONS_V6.document_type_versions
    },
    ..CONTRACT_VERSIONS_V6
};
