use versioned_feature_core::FeatureVersion;

pub mod v1;
pub mod v2;
pub mod v3;

#[derive(Clone, Debug, Default)]
pub struct DPPVotingVersions {
    pub default_vote_poll_time_duration_mainnet_ms: u64,
    pub default_vote_poll_time_duration_test_network_ms: u64,
    pub contested_document_vote_poll_stored_info_version: FeatureVersion,
    /// Structure version `ReadinessRound::new` builds: the compilation readiness round Drive
    /// keeps per contract from protocol version 17. Shipped tables carry 0 because the record
    /// did not exist before; nothing reads it there.
    pub readiness_round_default_structure_version: FeatureVersion,
    /// Structure version `ReadinessReportRecord::new` builds: one accepted readiness report
    /// under a round's count tree. Shipped tables carry 0 for the same reason.
    pub readiness_report_record_default_structure_version: FeatureVersion,
    /// Structure version `ReadinessScanCursor::new` builds: the persisted position of a paged
    /// eligibility walk bound to one membership view. Shipped tables carry 0 for the same reason.
    pub readiness_scan_cursor_default_structure_version: FeatureVersion,
}
