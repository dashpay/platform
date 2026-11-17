use crate::version::drive_versions::drive_state_transition_method_versions::v4::DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_state_transition_method_versions::{
    DriveStateTransitionActionConvertToHighLevelOperationsMethodVersions,
    DriveStateTransitionMethodVersions,
};

/// Introduced in protocol v15: `document_create_transition` 1 deletes each document a create
/// consumes from what document reference validation 1 read of it, without reading it again.
/// `document_delete_transition` 1 carries the deleter for a retained-history lifecycle
/// record. `document_erase_transition` is `Some(0)`, converting bounded erasures into Drive
/// operations. Every other selector matches v4.
pub const DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5: DriveStateTransitionMethodVersions =
    DriveStateTransitionMethodVersions {
        convert_to_high_level_operations:
            DriveStateTransitionActionConvertToHighLevelOperationsMethodVersions {
                document_delete_transition: 1, // changed: carries the lifecycle record owner
                document_erase_transition: Some(0), // changed: converts a bounded retained-history erase
                document_create_transition: 1, // changed: a consume deletes what its lookup read
                ..DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4.convert_to_high_level_operations
            },
        ..DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4
    };
