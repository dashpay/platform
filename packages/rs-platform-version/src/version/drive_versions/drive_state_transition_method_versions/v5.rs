use crate::version::drive_versions::drive_state_transition_method_versions::v4::DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_state_transition_method_versions::{
    DriveStateTransitionActionConvertToHighLevelOperationsMethodVersions,
    DriveStateTransitionMethodVersions,
};

/// Protocol 15: `DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4` with
/// `token_destroy_frozen_funds_transition` 1, which destroys what a frozen identity's open
/// listings hold with its balance.
pub const DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V5: DriveStateTransitionMethodVersions =
    DriveStateTransitionMethodVersions {
        convert_to_high_level_operations:
            DriveStateTransitionActionConvertToHighLevelOperationsMethodVersions {
                token_destroy_frozen_funds_transition: 1, // changed: also destroys the frozen identity's escrowed listings
                ..DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4.convert_to_high_level_operations
            },
        ..DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V4
    };
