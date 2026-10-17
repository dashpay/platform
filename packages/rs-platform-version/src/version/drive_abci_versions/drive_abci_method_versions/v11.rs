use super::v10::DRIVE_ABCI_METHOD_VERSIONS_V10;
use super::{DriveAbciCoreChainLockMethodVersionsAndConstants, DriveAbciMethodVersions};

/// Drive ABCI method versions 11, introduced in protocol v15. Differs from v10 in one slot.
/// `core_chain_lock.verify_chain_lock_locally` is 1 rather than 0: a chain lock is rejected
/// locally only when Platform holds exactly the quorums active at its signing height. When a
/// quorum may have been mined after the last committed core height, or the signing height lies
/// between the two stored chain lock quorum sets, the chain lock is left to Core instead of
/// rejected, and Core's `verifychainlock` checks the signature of every chain lock left to Core,
/// since `submitchainlock` skips that check for a height Core already has a chain lock at or
/// above.
pub const DRIVE_ABCI_METHOD_VERSIONS_V11: DriveAbciMethodVersions = DriveAbciMethodVersions {
    core_chain_lock: DriveAbciCoreChainLockMethodVersionsAndConstants {
        verify_chain_lock_locally: 1,
        ..DRIVE_ABCI_METHOD_VERSIONS_V10.core_chain_lock
    },
    ..DRIVE_ABCI_METHOD_VERSIONS_V10
};
