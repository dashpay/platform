use crate::version::fee::processing::FeeProcessingVersion;

/// Explanation:
/// * min_leftover_credits_before_processing: This is the amount of credits we need to have left over in the identity to be able to do a
///     withdrawal or a credit transfer
pub const FEE_PROCESSING_VERSION1: FeeProcessingVersion = FeeProcessingVersion {
    fetch_identity_balance_processing_cost: 10000,
    fetch_identity_revision_processing_cost: 9000,
    fetch_identity_balance_and_revision_processing_cost: 15000,
    fetch_identity_cost_per_look_up_key_by_id: 9000,
    fetch_single_identity_key_processing_cost: 10000,
    validate_key_structure: 50,
    min_leftover_credits_before_processing: 100000, // not a fee, but still makes sense to put here
};
