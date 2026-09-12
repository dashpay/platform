use crate::version::fee::dashvm::v1::FEE_DASHVM_VERSION1;
use crate::version::fee::v2::FEE_VERSION2;
use crate::version::fee::FeeVersion;

/// Introduced in protocol version 17 (5.0).
///
/// Identical to [`FEE_VERSION2`] except that it prices smart-contract computation
/// (`dashvm: Some(FEE_DASHVM_VERSION1)`). Storage, processing, hashing and signature rates are
/// unchanged, so `fee_version_number` stays 1: the number keys the persisted fee history and
/// the storage refund rates, and a schedule that only changes a group the history never serves
/// keeps the number of the generation it agrees with. For the same reason this schedule is not
/// appended to `FEE_VERSIONS`, which holds one entry per number.
pub const FEE_VERSION3: FeeVersion = FeeVersion {
    dashvm: Some(FEE_DASHVM_VERSION1),
    ..FEE_VERSION2
};
