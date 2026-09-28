use crate::version::dpp_versions::dpp_token_versions::v3::TOKEN_VERSIONS_V3;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::drive_abci_method_versions::v11::DRIVE_ABCI_METHOD_VERSIONS_V11;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::v10::DRIVE_VERSION_V10;
use crate::version::protocol_version::PlatformVersion;
use crate::version::v16::PLATFORM_V16;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_17: ProtocolVersion = 17;

/// The 5.0 protocol version, the one that introduces smart contracts. Provisional number from
/// the DashVM allocation register (15 for 4.3, 16 for 4.4, 17 for 5.0).
///
/// Changes over v16 so far:
///
/// * `DRIVE_VERSION_V10` adds the per-issuer token lifecycle ledger under `[Tokens] / 224`:
///   one record per contract that issues tokens holding its supply rollup and, once an issuer
///   is destroyed, the wipe marker; a destroyed supply scalar at `[0]` that excludes wiped
///   issuers from the active totals without touching a holder leaf; and the cleanup queue tree
///   at `[1]`. `create_initial_state_structure` 3 -> 4 creates the ledger at genesis, the first
///   block at this version creates it on upgraded nodes and backfills one record per issuer
///   after reconciling every token's supply with its balance sum, every native supply write
///   keeps the rollup current, and `calculate_total_tokens_balance` 0 -> 1 checks raw and
///   active totals against the ledger.
/// * `DRIVE_ABCI_METHOD_VERSIONS_V11` selects generation 2 of the protocol change hook, the
///   one that runs the ledger transition on upgraded nodes.
/// * `TOKEN_VERSIONS_V3` adds the default structure version of the lifecycle record.
///
/// The ledger key and its sub keys are provisional (the allocation register leaves new keys
/// under `[Tokens]` unallocated) and are revised, if at all, before any network is asked to
/// propose this version.
pub const PLATFORM_V17: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_17,
    drive: DRIVE_VERSION_V10, // changed: per-issuer token supply rollups and the destroyed-issuer ledger
    drive_abci: DriveAbciVersion {
        methods: DRIVE_ABCI_METHOD_VERSIONS_V11, // changed: protocol change hook generation 2 runs the ledger transition
        ..PLATFORM_V16.drive_abci
    },
    dpp: DPPVersion {
        token_versions: TOKEN_VERSIONS_V3, // changed: contract token lifecycle default structure version
        ..PLATFORM_V16.dpp
    },
    ..PLATFORM_V16
};
