use crate::version::drive_versions::v10::DRIVE_VERSION_V10;
use crate::version::protocol_version::PlatformVersion;
use crate::version::v16::PLATFORM_V16;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_17: ProtocolVersion = 17;

/// The 5.0 protocol version, the one that introduces smart contracts. Provisional number from
/// the DashVM allocation register (15 for 4.3, 16 for 4.4, 17 for 5.0).
///
/// One change over v16 so far:
///
/// * `DRIVE_VERSION_V10` adds the contract credits root sum tree (`RootTree::ContractCredits`,
///   key 100) to the state: `create_initial_state_structure` 3 -> 4 creates it at genesis, the
///   first block at this version creates it on upgraded nodes, and
///   `calculate_total_credits_balance` 2 -> 3 reads it as the sixth term of the credit
///   conservation equation. Contract credit buckets, their rules and their proofs arrive with
///   later changes; until then the tree stays empty and the term is zero.
///
/// The root key value is provisional (the allocation register leaves new root values
/// unallocated) and is revised, if at all, before any network is asked to propose this
/// version.
pub const PLATFORM_V17: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_17,
    drive: DRIVE_VERSION_V10, // changed: contract credits root sum tree at genesis, on upgrade and in credit conservation
    ..PLATFORM_V16
};
