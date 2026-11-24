use crate::version::dpp_versions::dpp_document_versions::v4::DOCUMENT_VERSIONS_V4;
use crate::version::dpp_versions::dpp_document_versions::{
    DPPDocumentVersions, DocumentMethodVersions,
};

/// Introduced in protocol version 15. Differs from v4 in:
/// - `id_and_revision_from_bytes`, `Some(0)` rather than `None`: the migration of retained
///   document histories on the first block of the version reads each retained revision's id
///   and revision from the fields formats 0 to 3 write ahead of the document's properties.
///
/// v4 remains unchanged for `PROTOCOL_VERSION_14`.
pub const DOCUMENT_VERSIONS_V5: DPPDocumentVersions = DPPDocumentVersions {
    document_method_versions: DocumentMethodVersions {
        id_and_revision_from_bytes: Some(0), // new: a retained revision's id and revision read without its properties
        ..DOCUMENT_VERSIONS_V4.document_method_versions
    },
    ..DOCUMENT_VERSIONS_V4
};
