use crate::version::consensus_versions::ConsensusVersions;
use crate::version::drive_abci_versions::drive_abci_method_versions::v11::DRIVE_ABCI_METHOD_VERSIONS_V11;
use crate::version::drive_abci_versions::drive_abci_query_versions::v4::DRIVE_ABCI_QUERY_VERSIONS_V4;
use crate::version::drive_abci_versions::DriveAbciVersion;
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
/// * `SYSTEM_LIMITS_V5` bounds the contract-code capable generations of the contract create and
///   update transitions: 32 MiB on the wire, a 64 MiB decode budget, 16 modules and 16 MiB of
///   canonical code per bundle. Every other family keeps the 20 KiB cap.
/// * `DRIVE_ABCI_METHOD_VERSIONS_V11` selects `decode_raw_state_transitions` v1, which reads the
///   family cap from the wire prefix before decoding, and `consensus_params_update` v2, which
///   pushes the block parameters below to Tenderdash at the activation boundary.
/// * `DRIVE_ABCI_QUERY_VERSIONS_V4` selects `proofs_query` v1, which decodes the transition of a
///   proof request under the same family budget.
/// * `consensus.block_max_bytes` and `block_max_gas` are set: one maximal contract envelope plus
///   4 MiB for the header, commit and evidence, and the gas value every network launched with,
///   carried so the push does not zero it.
///
/// All numbers are provisional register values, measured and revised before any network is
/// asked to propose this version. No generation that produces a contract-code envelope exists
/// yet, so a node at v17 accepts exactly what one at v16 accepts; the tables are the
/// consensus-side scaffolding for the generation that follows.
pub const PLATFORM_V17: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_17,
    drive_abci: DriveAbciVersion {
        methods: DRIVE_ABCI_METHOD_VERSIONS_V11, // changed: family-cap decode generation + block consensus parameter push
        query: DRIVE_ABCI_QUERY_VERSIONS_V4, // changed: getProofs decodes under the family budget
        ..PLATFORM_V16.drive_abci
    },
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 1,
        // provisional: one 32 MiB contract envelope plus 4 MiB of header, commit and evidence
        block_max_bytes: Some(37_748_736),
        // the value every network launched with (dashmate genesis `block.max_gas`), carried so
        // the block parameter push keeps it
        block_max_gas: Some(57_631_392_000),
    },
    system_limits: SYSTEM_LIMITS_V5, // changed: contract-code envelope, bundle and module limits
    ..PLATFORM_V16
};
