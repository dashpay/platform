use crate::version::dpp_versions::dpp_voting_versions::v3::VOTING_VERSION_V3;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::drive_abci_method_versions::v11::DRIVE_ABCI_METHOD_VERSIONS_V11;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::v10::DRIVE_VERSION_V10;
use crate::version::protocol_version::PlatformVersion;
use crate::version::system_limits::v5::SYSTEM_LIMITS_V5;
use crate::version::v16::PLATFORM_V16;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_17: ProtocolVersion = 17;

/// The 5.0 protocol version, the one that introduces smart contracts. Provisional number from
/// the DashVM allocation register (15 for 4.3, 16 for 4.4, 17 for 5.0).
///
/// Changes over v16 so far:
///
/// * `DRIVE_VERSION_V10` adds compilation readiness storage: rounds keyed under
///   `[Votes] / r / 0 / contract_id / round_id` with a current-round pointer, a count tree of
///   accepted reports per round, the paged scan cursor, the activation deadline queue at
///   `[Votes] / r / 1`, the block event's fairness cursor at `[Votes] / r / 2`, the retired
///   round cleanup queue at `[Votes] / r / 3`, and the readiness funds sum tree at
///   `[PreFundedSpecializedBalances] / 129` beside the voting funds. Genesis creates them
///   through `add_initial_vote_tree_main_structure_operations` generation 1; three verifiers
///   prove a round, a report and a fund.
/// * `DRIVE_ABCI_METHOD_VERSIONS_V11` selects generation 3 of the protocol change hook, which
///   creates the same structures on upgraded nodes.
/// * `VOTING_VERSION_V3` declares the structure versions of the round, report record and
///   scan cursor.
/// * `SYSTEM_LIMITS_V5` bounds the additional wait before activation to two minutes and one
///   hour.
///
/// The subtree keys, the fund purpose key and the wait placement are provisional (the
/// allocation register leaves new inner tags unallocated) and are revised, if at all, before
/// any network is asked to propose this version.
pub const PLATFORM_V17: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_17,
    drive: DRIVE_VERSION_V10, // changed: compilation readiness storage, genesis setup and verifiers
    drive_abci: DriveAbciVersion {
        methods: DRIVE_ABCI_METHOD_VERSIONS_V11, // changed: protocol change hook generation 3 creates the readiness structures
        ..PLATFORM_V16.drive_abci
    },
    dpp: DPPVersion {
        voting_versions: VOTING_VERSION_V3, // changed: readiness round, report record and scan cursor structure versions
        ..PLATFORM_V16.dpp
    },
    system_limits: SYSTEM_LIMITS_V5, // changed: readiness additional wait bounds
    ..PLATFORM_V16
};
