use crate::version::dpp_versions::dpp_voting_versions::DPPVotingVersions;

pub const VOTING_VERSION_V2: DPPVotingVersions = DPPVotingVersions {
    default_vote_poll_time_duration_mainnet_ms: 1_209_600_000, //2 weeks
    default_vote_poll_time_duration_test_network_ms: 5_400_000, //90 minutes
    contested_document_vote_poll_stored_info_version: 0,
    readiness_round_default_structure_version: 0,
    readiness_report_record_default_structure_version: 0,
    readiness_scan_cursor_default_structure_version: 0,
};
