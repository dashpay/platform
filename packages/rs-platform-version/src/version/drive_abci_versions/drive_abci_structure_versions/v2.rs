use crate::version::drive_abci_versions::drive_abci_structure_versions::DriveAbciStructureVersions;

/// Changed from V1: the saved platform state is structure 1 (`PlatformStateForSavingV2`),
/// which keeps the masternode list and the validator sets as one aux entry each
/// instead of inside the record.
pub const DRIVE_ABCI_STRUCTURE_VERSIONS_V2: DriveAbciStructureVersions =
    DriveAbciStructureVersions {
        platform_state_structure: 0,
        platform_state_for_saving_structure_default: 1,
        state_transition_execution_context: 0,
        commit: 0,
        masternode: 0,
        signature_verification_quorum_set: 0,
    };
