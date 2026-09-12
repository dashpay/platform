use crate::version::drive_abci_versions::drive_abci_method_versions::v10::DRIVE_ABCI_METHOD_VERSIONS_V10;
use crate::version::drive_abci_versions::drive_abci_method_versions::{
    DriveAbciEngineMethodVersions, DriveAbciMethodVersions,
    DriveAbciStateTransitionProcessingMethodVersions,
};

/// Drive ABCI method versions 11. Introduced in protocol version 17 (5.0).
///
/// Two slots change over `DRIVE_ABCI_METHOD_VERSIONS_V10`:
///
/// * `engine.consensus_params_update` becomes 2: besides the version parameters v1 pushes, the
///   new generation pushes `ConsensusParams.block` (`max_bytes`, `max_gas`) whenever the new
///   protocol version sets `ConsensusVersions::block_max_bytes` to a value the previous one did
///   not carry, so every validator raises the Tenderdash block size at the same height.
/// * `state_transition_processing.decode_raw_state_transitions` becomes 1: the raw length is
///   compared with the cap of the family the wire prefix names (the contract-code capable
///   generations read `SystemLimits::max_contract_code_state_transition_size`, everything else
///   `max_state_transition_size`), the envelope is decoded under that family's bincode budget,
///   and a variant the active version does not admit is classified as an unpaid consensus
///   rejection rather than a node fault.
///
/// Everything else matches v10.
pub const DRIVE_ABCI_METHOD_VERSIONS_V11: DriveAbciMethodVersions = DriveAbciMethodVersions {
    engine: DriveAbciEngineMethodVersions {
        consensus_params_update: 2, // changed in v17: pushes the block byte and gas caps at the activation boundary
        ..DRIVE_ABCI_METHOD_VERSIONS_V10.engine
    },
    state_transition_processing: DriveAbciStateTransitionProcessingMethodVersions {
        decode_raw_state_transitions: 1, // changed in v17: family-specific size cap and decode budget from the wire prefix
        ..DRIVE_ABCI_METHOD_VERSIONS_V10.state_transition_processing
    },
    ..DRIVE_ABCI_METHOD_VERSIONS_V10
};
