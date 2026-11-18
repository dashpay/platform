use crate::execution::types::execution_operation::ValidationOperation;

/// What check tx reserves, on top of a transition's estimated fee, for work a block bills where
/// check tx validated no state, in two parts. Fee validation holds the payer to both; check tx
/// counts only `billed` in the gas it reports the transition as wanting, which Tenderdash
/// fills a block's gas with.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(in crate::execution) struct CheckTxReservation {
    /// Work a block bills whenever the transition is processed as expected: a `refersTo`
    /// lookup's key hash and the delete of the document it consumes, a `chooseFirst` check's
    /// own hashes, an erase's refund work.
    pub(in crate::execution) billed: Vec<ValidationOperation>,
    /// Bounds on work a block bills at most as much as: a lookup query's allowance, the
    /// `chooseFirst` competitor scan's budget and its competitors' hashes.
    pub(in crate::execution) bounds: Vec<ValidationOperation>,
}

impl CheckTxReservation {
    /// Adds `other`'s operations to this reservation's.
    pub(in crate::execution) fn extend(&mut self, other: CheckTxReservation) {
        self.billed.extend(other.billed);
        self.bounds.extend(other.bounds);
    }

    /// Every operation reserved, billed ones first.
    pub(in crate::execution) fn into_operations(self) -> Vec<ValidationOperation> {
        let mut operations = self.billed;
        operations.extend(self.bounds);
        operations
    }
}
