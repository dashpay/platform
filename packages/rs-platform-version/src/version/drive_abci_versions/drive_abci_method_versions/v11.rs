use super::v10::DRIVE_ABCI_METHOD_VERSIONS_V10;
use super::{
    DkgMiningSchedule, DriveAbciCoreChainLockMethodVersionsAndConstants, DriveAbciMethodVersions,
    DriveAbciProtocolUpgradeMethodVersions,
};

/// Dash Core's DKG schedule (`llmq/params.h`) of the non-rotating quorum types, the ones chain
/// locks are signed with. Frozen here rather than read from the dashcore dependency, so a
/// dependency update cannot change which chain locks are rejected locally.
const DKG_MINING_SCHEDULES_V1: &[DkgMiningSchedule] = &[
    // llmq_50_60
    DkgMiningSchedule {
        quorum_type: 1,
        interval: 24,
        mining_window_start: 10,
        mining_window_end: 18,
    },
    // llmq_400_60
    DkgMiningSchedule {
        quorum_type: 2,
        interval: 288,
        mining_window_start: 20,
        mining_window_end: 28,
    },
    // llmq_400_85
    DkgMiningSchedule {
        quorum_type: 3,
        interval: 576,
        mining_window_start: 20,
        mining_window_end: 48,
    },
    // llmq_100_67
    DkgMiningSchedule {
        quorum_type: 4,
        interval: 24,
        mining_window_start: 10,
        mining_window_end: 18,
    },
    // llmq_25_67
    DkgMiningSchedule {
        quorum_type: 6,
        interval: 24,
        mining_window_start: 10,
        mining_window_end: 18,
    },
    // llmq_test
    DkgMiningSchedule {
        quorum_type: 100,
        interval: 24,
        mining_window_start: 10,
        mining_window_end: 18,
    },
    // llmq_devnet
    DkgMiningSchedule {
        quorum_type: 101,
        interval: 24,
        mining_window_start: 10,
        mining_window_end: 18,
    },
    // llmq_test_v17
    DkgMiningSchedule {
        quorum_type: 102,
        interval: 24,
        mining_window_start: 10,
        mining_window_end: 18,
    },
    // llmq_test_instantsend
    DkgMiningSchedule {
        quorum_type: 104,
        interval: 24,
        mining_window_start: 10,
        mining_window_end: 18,
    },
    // llmq_test_platform
    DkgMiningSchedule {
        quorum_type: 106,
        interval: 24,
        mining_window_start: 10,
        mining_window_end: 18,
    },
    // llmq_devnet_platform
    DkgMiningSchedule {
        quorum_type: 107,
        interval: 24,
        mining_window_start: 10,
        mining_window_end: 18,
    },
];

/// Drive ABCI method versions 11, introduced in protocol v15. Differs from v10 in the chain lock
/// verification, which only works as a whole:
/// - `core_chain_lock.verify_chain_lock_locally` is 1 rather than 0: a chain lock is rejected
///   locally only when Platform holds exactly the quorums active at its signing height, using
///   Dash Core's DKG mining schedule in `core_chain_lock.dkg_mining_schedules`. When a quorum may
///   have been mined after the last committed core height, or the signing height lies between
///   the two stored chain lock quorum sets, the chain lock is left to Core instead of rejected.
/// - `core_chain_lock.verify_chain_lock_through_core` is 1 rather than 0: Core's
///   `verifychainlock` checks the signature of every chain lock left to Core before
///   `submitchainlock`, which skips that check for a height Core already has a chain lock at or
///   above. `verify_chain_lock_locally` 1 leaves to Core chain locks that 0 rejected, so it is
///   only safe together with this check.
pub const DRIVE_ABCI_METHOD_VERSIONS_V11: DriveAbciMethodVersions = DriveAbciMethodVersions {
    protocol_upgrade: DriveAbciProtocolUpgradeMethodVersions {
        perform_events_on_first_block_of_protocol_change: Some(3),
        ..DRIVE_ABCI_METHOD_VERSIONS_V10.protocol_upgrade
    },
    core_chain_lock: DriveAbciCoreChainLockMethodVersionsAndConstants {
        verify_chain_lock_locally: 1,
        verify_chain_lock_through_core: 1,
        dkg_mining_schedules: Some(DKG_MINING_SCHEDULES_V1),
        ..DRIVE_ABCI_METHOD_VERSIONS_V10.core_chain_lock
    },
    ..DRIVE_ABCI_METHOD_VERSIONS_V10
};
