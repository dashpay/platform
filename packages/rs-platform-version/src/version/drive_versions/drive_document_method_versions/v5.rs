use crate::version::drive_versions::drive_document_method_versions::v4::DRIVE_DOCUMENT_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_document_method_versions::DriveDocumentMethodVersions;

/// V5 is protocol version 15's document-method table. Relative to V4,
/// `insert_empty_private_document_store_operation` is `Some(0)` (`None` in every earlier
/// table): a contract registering a private document type, or an update adding one, creates
/// the type's private document store at its key `[0]`, sized from the private document store
/// limits of `SYSTEM_LIMITS_V5`.
pub const DRIVE_DOCUMENT_METHOD_VERSIONS_V5: DriveDocumentMethodVersions =
    DriveDocumentMethodVersions {
        insert_empty_private_document_store_operation: Some(0), // changed: a private document type's store is created with it
        ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4
    };
