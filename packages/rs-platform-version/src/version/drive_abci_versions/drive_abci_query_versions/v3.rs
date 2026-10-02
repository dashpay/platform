use crate::version::drive_abci_versions::drive_abci_query_versions::v2::DRIVE_ABCI_QUERY_VERSIONS_V2;
use crate::version::drive_abci_versions::drive_abci_query_versions::DriveAbciQueryVersions;

/// Version 3 of the Drive ABCI query versions. Introduced in protocol version 17.
///
/// Differs from v2 in exactly one slot: `proofs_query` is 1 rather than 0. The v0 handler
/// decodes the transition carried by a `getProofs` request through the untrusted decoder,
/// which applies no bincode byte budget of its own (the `limit` attribute on
/// `StateTransition` is inert); the v1 handler peeks the envelope kind first and decodes a
/// contract-code capable envelope under the family budget of the active version
/// (`SystemLimits::max_contract_code_state_transition_decode_budget`), so a proof request for
/// a transition the block accepted is never rejected for its size. Ordinary transitions decode
/// exactly as before.
pub const DRIVE_ABCI_QUERY_VERSIONS_V3: DriveAbciQueryVersions = DriveAbciQueryVersions {
    proofs_query: 1,
    ..DRIVE_ABCI_QUERY_VERSIONS_V2
};
