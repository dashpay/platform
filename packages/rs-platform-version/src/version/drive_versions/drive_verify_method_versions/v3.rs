use super::v2::DRIVE_VERIFY_METHOD_VERSIONS_V2;
use super::{DriveVerifyDocumentMethodVersions, DriveVerifyMethodVersions};

/// Verification of composite document history and its independent metadata proof.
pub const DRIVE_VERIFY_METHOD_VERSIONS_V3: DriveVerifyMethodVersions = DriveVerifyMethodVersions {
    document: DriveVerifyDocumentMethodVersions {
        verify_document_history: 1,
        ..DRIVE_VERIFY_METHOD_VERSIONS_V2.document
    },
    ..DRIVE_VERIFY_METHOD_VERSIONS_V2
};
