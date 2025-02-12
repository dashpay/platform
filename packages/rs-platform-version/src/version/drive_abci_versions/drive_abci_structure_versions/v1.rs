use crate::version::drive_abci_versions::drive_abci_structure_versions::DriveAbciStructureVersions;

pub const DRIVE_ABCI_STRUCTURE_VERSIONS_V1: DriveAbciStructureVersions =
    DriveAbciStructureVersions {
        platform_state_structure: 0,
        platform_state_for_saving_structure_default: 0,
        reduced_platform_state_for_saving_structure_default: 0,
        state_transition_execution_context: 0,
        commit: 0,
        masternode: 0,
        signature_verification_quorum_set: 0,
    };
