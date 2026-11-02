use crate::version::drive_versions::drive_state_transition_method_versions::v4::DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_state_transition_method_versions::{
    DriveStateTransitionActionConvertToHighLevelOperationsMethodVersions,
    DriveStateTransitionMethodVersions,
};

/// Introduced in protocol v15: `document_create_transition` 1 deletes each document a create
/// consumes from what document reference validation 1 read of it, without reading it again.
/// Everything else is v4's.
pub const DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5: DriveStateTransitionMethodVersions =
    DriveStateTransitionMethodVersions {
        convert_to_high_level_operations:
            DriveStateTransitionActionConvertToHighLevelOperationsMethodVersions {
                document_create_transition: 1, // changed: a consume deletes what its lookup read
                ..DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4.convert_to_high_level_operations
            },
        ..DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4
    };
