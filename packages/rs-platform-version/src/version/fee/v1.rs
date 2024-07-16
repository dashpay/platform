use crate::version::fee::data_contract::v1::FEE_DATA_CONTRACT_VALIDATION_VERSION1;
use crate::version::fee::hashing::v1::FEE_HASHING_VERSION1;
use crate::version::fee::processing::v1::FEE_PROCESSING_VERSION1;
use crate::version::fee::signature::v1::FEE_SIGNATURE_VERSION1;
use crate::version::fee::state_transition_min_fees::v1::STATE_TRANSITION_MIN_FEES_VERSION1;
use crate::version::fee::storage::v1::FEE_STORAGE_VERSION1;
use crate::version::fee::vote_resolution_fund_fees::v1::VOTE_RESOLUTION_FUND_FEES_VERSION1;
use crate::version::fee::FeeVersion;

pub const FEE_VERSION1: FeeVersion = FeeVersion {
    uses_version_fee_multiplier_permille: Some(1000), //No action
    storage: FEE_STORAGE_VERSION1,
    signature: FEE_SIGNATURE_VERSION1,
    hashing: FEE_HASHING_VERSION1,
    processing: FEE_PROCESSING_VERSION1,
    data_contract: FEE_DATA_CONTRACT_VALIDATION_VERSION1,
    state_transition_min_fees: STATE_TRANSITION_MIN_FEES_VERSION1,
    vote_resolution_fund_fees: VOTE_RESOLUTION_FUND_FEES_VERSION1,
};
