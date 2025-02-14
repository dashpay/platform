use crate::version::fee::processing::FeeProcessingVersion;

pub const FEE_PROCESSING_VERSION1: FeeProcessingVersion = FeeProcessingVersion {
    fetch_identity_balance_processing_cost: 10000,
    fetch_identity_revision_processing_cost: 9000,
    fetch_identity_balance_and_revision_processing_cost: 15000,
    fetch_identity_cost_per_look_up_key_by_id: 9000,
    fetch_identity_token_balance_processing_cost: 10000,
    fetch_prefunded_specialized_balance_processing_cost: 10000,
    fetch_single_identity_key_processing_cost: 10000,
    perform_network_threshold_signing: 100000000, // 1mDash (2.5 cents at 25$/Dash)
    validate_key_structure: 50,
};
