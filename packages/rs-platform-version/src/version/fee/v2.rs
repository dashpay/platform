use crate::version::fee::data_contract_registration::v2::FEE_DATA_CONTRACT_REGISTRATION_VERSION2;
use crate::version::fee::data_contract_validation::v1::FEE_DATA_CONTRACT_VALIDATION_VERSION1;
use crate::version::fee::hashing::v1::FEE_HASHING_VERSION1;
use crate::version::fee::processing::v1::FEE_PROCESSING_VERSION1;
use crate::version::fee::signature::v1::FEE_SIGNATURE_VERSION1;
use crate::version::fee::state_transition_min_fees::v1::STATE_TRANSITION_MIN_FEES_VERSION1;
use crate::version::fee::storage::v1::FEE_STORAGE_VERSION1;
use crate::version::fee::vote_resolution_fund_fees::v1::VOTE_RESOLUTION_FUND_FEES_VERSION1;
use crate::version::fee::FeeVersion;

/// Introduced in protocol version 9 (2.0)
///
/// BUG(#4647): `fee_version_number` collides with `FEE_VERSION1` and this constant is missing
/// from `FEE_VERSIONS`, so `FeeVersion::get(1)` never resolves to it. The number is the
/// only thing persisted (`PlatformStateForSavingV1`, `ReducedPlatformStateV0`), so a
/// node that restarts or state-syncs rehydrates previous epochs' fees as `FEE_VERSION1`.
/// Latent only because the two share identical storage fees, which is all
/// `previous_fee_versions` is consulted for. Giving it number 2 is protocol-visible and
/// needs a versioned migration; see `fee_version_numbers_are_unique_and_resolvable`.
pub const FEE_VERSION2: FeeVersion = FeeVersion {
    fee_version_number: 1, // BUG: must be 2, see the doc comment above
    uses_version_fee_multiplier_permille: Some(1000), //No action
    storage: FEE_STORAGE_VERSION1,
    signature: FEE_SIGNATURE_VERSION1,
    hashing: FEE_HASHING_VERSION1,
    processing: FEE_PROCESSING_VERSION1,
    data_contract_validation: FEE_DATA_CONTRACT_VALIDATION_VERSION1,
    data_contract_registration: FEE_DATA_CONTRACT_REGISTRATION_VERSION2, // changed to v2
    state_transition_min_fees: STATE_TRANSITION_MIN_FEES_VERSION1,
    vote_resolution_fund_fees: VOTE_RESOLUTION_FUND_FEES_VERSION1,
};
