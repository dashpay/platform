use crate::version::drive_versions::drive_document_method_versions::{
    DriveDocumentDeleteMethodVersions, DriveDocumentMethodVersions,
};

use super::v4::DRIVE_DOCUMENT_METHOD_VERSIONS_V4;

/// Protocol v15 document method versions: `delete_consumed_document_operations` 1, a create
/// deletes the document it consumes from what its `refersTo` lookup read, without reading it
/// again.
pub const DRIVE_DOCUMENT_METHOD_VERSIONS_V5: DriveDocumentMethodVersions =
    DriveDocumentMethodVersions {
        delete: DriveDocumentDeleteMethodVersions {
            delete_consumed_document_operations: 1,
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.delete
        },
        ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4
    };
