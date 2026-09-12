use dpp::fee::Credits;
use dpp::prelude::Identifier;
use std::collections::BTreeMap;

/// What `Drive::credit_storage_refunds_to_owners_operations` did with a set of
/// storage refunds: which recorded owners were credited and how much could not
/// be routed to any owner.
///
/// The caller that knows the block's epoch settles `routed_to_processing_pool`
/// into that epoch's processing pool with a single pool write and records the
/// refunds against their storage epochs; the primitive itself never touches
/// the pools.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StorageRefundCreditOutcome {
    /// Credits added to each recorded owner's balance, summed over the epochs
    /// the owner's bytes were stored in.
    pub credited: BTreeMap<Identifier, Credits>,
    /// Credits whose recorded owner has no balance element. Until a typed
    /// owner encoding exists this is the native proxy for a wiped owner, so
    /// the caller moves this amount into the current epoch's processing pool.
    pub routed_to_processing_pool: Credits,
}

impl StorageRefundCreditOutcome {
    /// The total credited to owners plus the amount routed to the pool.
    pub fn total(&self) -> Option<Credits> {
        self.credited
            .values()
            .try_fold(self.routed_to_processing_pool, |total, credits| {
                total.checked_add(*credits)
            })
    }
}
