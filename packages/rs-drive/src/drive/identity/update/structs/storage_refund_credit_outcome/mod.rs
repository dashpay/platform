use dpp::fee::Credits;
use dpp::prelude::Identifier;
use std::collections::BTreeMap;

/// What `Drive::credit_storage_refunds_to_owners_operations` did with a set of
/// storage refunds: which recorded owners were credited, how much of the
/// refunds cleared identity debt instead of reaching a balance, and how much
/// could not be routed to any owner.
///
/// The caller that knows the block's epoch settles `processing_pool_share()`
/// into that epoch's processing pool with a single pool write and records the
/// refunds against their storage epochs; the primitive itself never touches
/// the pools.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StorageRefundCreditOutcome {
    /// Credits that reached each recorded owner's balance element, summed
    /// over the epochs the owner's bytes were stored in. An owner whose whole
    /// refund went into clearing debt has no entry.
    pub credited: BTreeMap<Identifier, Credits>,
    /// Credits that cleared an owner's negative credit (identity debt) instead
    /// of raising its balance. Debt is processing fee the pools were short of
    /// when it was incurred and it lives outside the credit sum trees, so the
    /// caller moves this amount into the current epoch's processing pool to
    /// keep the credit sum balanced.
    pub repaid_debt: Credits,
    /// Credits whose recorded owner has no balance element. Until a typed
    /// owner encoding exists this is the native proxy for a wiped owner, so
    /// the caller moves this amount into the current epoch's processing pool.
    pub routed_to_processing_pool: Credits,
}

impl StorageRefundCreditOutcome {
    /// What the caller writes into the current epoch's processing pool: the
    /// unrouted refunds plus the debt they repaid.
    pub fn processing_pool_share(&self) -> Option<Credits> {
        self.routed_to_processing_pool.checked_add(self.repaid_debt)
    }

    /// The total settled: credited to balances, repaid as debt, or routed to
    /// the pool.
    pub fn total(&self) -> Option<Credits> {
        self.credited
            .values()
            .try_fold(self.processing_pool_share()?, |total, credits| {
                total.checked_add(*credits)
            })
    }
}
