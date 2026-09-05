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
/// # WARNING: `fee_version_number` collides with [`FEE_VERSION1`], and this one is not
/// reachable by number
///
/// [`FeeVersion::get`] resolves a number through [`FEE_VERSIONS`], which contains only
/// `FEE_VERSION1`. This constant declares the SAME `fee_version_number: 1`, so
/// `FeeVersion::get(1)` can only ever return `FEE_VERSION1` — never this one, even though
/// this is what protocol versions 9 and later actually run with, and the two differ in
/// `data_contract_registration`.
///
/// That makes every number-only round trip of a fee version silently lossy. Two exist:
///
/// * `PlatformStateForSavingV1` stores `previous_fee_versions` as
///   `(epoch index -> fee version number)`, so a node that RESTARTS rehydrates previous
///   epochs' fees as `FEE_VERSION1`;
/// * `ReducedPlatformStateV0` does the same, so a node that STATE-SYNCS gets the same
///   substitution without even restarting.
///
/// It is latent rather than a live consensus fork only because `previous_fee_versions` is
/// consulted solely to price storage refunds (`rs-drive/src/fees/op.rs`), and
/// `FEE_VERSION1` and `FEE_VERSION2` have IDENTICAL `storage` fees. It becomes a fork the
/// moment a future `FeeVersion` changes a storage or processing fee without also taking a
/// distinct number.
///
/// ## The rule
///
/// **Every `FeeVersion` constant must have a unique `fee_version_number`, and must be
/// listed in [`FEE_VERSIONS`] at the index its number implies.** Fixing this constant to
/// `fee_version_number: 2` and adding it to `FEE_VERSIONS` is protocol-visible (it changes
/// what a restarted or state-synced node computes for old epochs), so it needs a
/// versioned migration rather than an in-place edit — which is why this is documented here
/// instead of changed. `fee_version_numbers_are_unique` in `super` is the enforcement, and
/// is `#[ignore]`d until then.
///
/// [`FEE_VERSION1`]: crate::version::fee::v1::FEE_VERSION1
/// [`FEE_VERSIONS`]: crate::version::fee::FEE_VERSIONS
/// [`FeeVersion::get`]: crate::version::fee::FeeVersion::get
pub const FEE_VERSION2: FeeVersion = FeeVersion {
    // BUG: must be 2. See the doc comment above — changing it is protocol-visible.
    fee_version_number: 1,
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
