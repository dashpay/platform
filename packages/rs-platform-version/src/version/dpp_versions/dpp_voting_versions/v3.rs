use crate::version::dpp_versions::dpp_voting_versions::v2::VOTING_VERSION_V2;
use crate::version::dpp_versions::dpp_voting_versions::DPPVotingVersions;

/// Introduced in protocol version 17 (5.0). Identical to [`VOTING_VERSION_V2`] except that the
/// three compilation readiness structures (the round, the accepted report record and the
/// paged scan cursor) have their default structure version declared.
pub const VOTING_VERSION_V3: DPPVotingVersions = DPPVotingVersions {
    readiness_round_default_structure_version: 0,
    readiness_report_record_default_structure_version: 0,
    readiness_scan_cursor_default_structure_version: 0,
    ..VOTING_VERSION_V2
};
