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
///
/// Two consequences follow, both intended. The epoch-change hook records a schedule in the fee
/// history only when its number changes, so a network upgrading to protocol version 17 keeps its
/// existing history entry, and a node restoring saved state resolves that entry to
/// [`FEE_VERSION1`](super::v1::FEE_VERSION1) through `FeeVersion::get(1)`. Neither carries the
/// `dashvm` group, and neither is meant to: contract pricing is read from the active protocol
/// version's schedule, like every other group the history does not serve. Registering this
/// schedule under a new number would not add the price to the history; it would switch every
/// storage refund at protocol version 17 onto the epoch-history refund path
/// (`fee_version_number != 1` in Drive's fee calculation) for rates that did not change.
pub const FEE_VERSION3: FeeVersion = FeeVersion {
    dashvm: Some(FEE_DASHVM_VERSION1),
    ..FEE_VERSION2
};

#[cfg(test)]
mod tests {
    use super::FEE_VERSION3;
    use crate::version::fee::FeeVersion;

    /// The schedule shares its number with a registered fee-history generation, so the epoch
    /// history and saved state resolve that number to the registered entry, not to this
    /// schedule. That is only sound while the two agree on every group the history serves. A
    /// change to a storage, processing, hashing or signature rate in this schedule needs a new
    /// registered number, and this test is what says so.
    #[test]
    fn should_agree_with_its_registered_fee_history_generation_on_every_group_the_history_serves() {
        let registered = FeeVersion::get(FEE_VERSION3.fee_version_number)
            .expect("the schedule's number is registered");

        assert_eq!(registered.storage, FEE_VERSION3.storage);
        assert_eq!(registered.processing, FEE_VERSION3.processing);
        assert_eq!(registered.hashing, FEE_VERSION3.hashing);
        assert_eq!(registered.signature, FEE_VERSION3.signature);

        // The history never carries contract pricing; consumers read it from the active
        // protocol version instead (see `dpp::fee::smart_contract_computation`).
        assert!(registered.dashvm.is_none());
        assert!(FEE_VERSION3.dashvm.is_some());
    }
}
