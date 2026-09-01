use crate::version::fee::data_contract_registration::v2::FEE_DATA_CONTRACT_REGISTRATION_VERSION2;
use crate::version::fee::data_contract_validation::v1::FEE_DATA_CONTRACT_VALIDATION_VERSION1;
use crate::version::fee::hashing::v1::FEE_HASHING_VERSION1;
use crate::version::fee::processing::v1::FEE_PROCESSING_VERSION1;
use crate::version::fee::signature::v1::FEE_SIGNATURE_VERSION1;
use crate::version::fee::state_transition_min_fees::v1::STATE_TRANSITION_MIN_FEES_VERSION1;
use crate::version::fee::storage::v2::FEE_STORAGE_VERSION2;
use crate::version::fee::vote_resolution_fund_fees::v1::VOTE_RESOLUTION_FUND_FEES_VERSION1;
use crate::version::fee::FeeVersion;

/// Introduced in protocol version 14: FEE_VERSION2 plus the TTL
/// ephemeral-bytes storage rate. Keeps `fee_version_number: 1`, the same
/// deliberate aliasing FEE_VERSION2 uses — the number tags the refund
/// algorithm and the rehydrated per-epoch refund fields, which are
/// identical across all three tables; the ttl rate is never read on the
/// refund path (ephemeral bytes create no refunds).
pub const FEE_VERSION3: FeeVersion = FeeVersion {
    fee_version_number: 1,
    uses_version_fee_multiplier_permille: Some(1000), //No action
    storage: FEE_STORAGE_VERSION2,
    signature: FEE_SIGNATURE_VERSION1,
    hashing: FEE_HASHING_VERSION1,
    processing: FEE_PROCESSING_VERSION1,
    data_contract_validation: FEE_DATA_CONTRACT_VALIDATION_VERSION1,
    data_contract_registration: FEE_DATA_CONTRACT_REGISTRATION_VERSION2, // changed to v2
    state_transition_min_fees: STATE_TRANSITION_MIN_FEES_VERSION1,
    vote_resolution_fund_fees: VOTE_RESOLUTION_FUND_FEES_VERSION1,
};
