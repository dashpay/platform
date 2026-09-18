use crate::version::drive_versions::drive_state_transition_method_versions::v4::DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_state_transition_method_versions::{
    DriveStateTransitionActionConvertToHighLevelOperationsMethodVersions,
    DriveStateTransitionMethodVersions,
};

/// Protocol v15 state transition methods for the keep-history document
/// lifecycle.
///
/// Delete conversion generation 1 records a lifecycle entry for a keep-history
/// document instead of removing its retained revisions; erase conversion
/// removes them a chunk at a time; batch conversion generation 1 consumes
/// batch action format 1, whose items may include an erase, and emits the
/// operations of every item.
pub const DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5: DriveStateTransitionMethodVersions =
    DriveStateTransitionMethodVersions {
        convert_to_high_level_operations:
            DriveStateTransitionActionConvertToHighLevelOperationsMethodVersions {
                document_delete_transition: 1,
                document_erase_transition: Some(0),
                documents_batch_transition: 1,
                ..DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4.convert_to_high_level_operations
            },
        ..DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4
    };
