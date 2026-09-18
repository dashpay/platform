use super::v2::DRIVE_VERIFY_METHOD_VERSIONS_V2;
use super::{
    DriveVerifyDocumentMethodVersions, DriveVerifyMethodVersions,
    DriveVerifyStateTransitionMethodVersions,
};

/// Verification of composite document history and its independent metadata proof,
/// and the cursor lookup of a proved page in the protocol 15 keep-history layout.
pub const DRIVE_VERIFY_METHOD_VERSIONS_V3: DriveVerifyMethodVersions = DriveVerifyMethodVersions {
    document: DriveVerifyDocumentMethodVersions {
        verify_document_history: 1,
        verify_start_at_document_in_proof: 1,
        ..DRIVE_VERIFY_METHOD_VERSIONS_V2.document
    },
    state_transition: DriveVerifyStateTransitionMethodVersions {
        verify_state_transition_was_executed_with_proof: 1,
    },
    ..DRIVE_VERIFY_METHOD_VERSIONS_V2
};
