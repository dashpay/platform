use crate::drive::Drive;
use crate::error::Error;
use grovedb::Transaction;

impl Drive {
    /// Commits a transaction.
    pub(crate) fn commit_transaction_v0(&self, transaction: Transaction) -> Result<(), Error> {
        self.grove
            .commit_transaction(transaction)
            .unwrap() // TODO: discuss what to do with transaction cost as costs are
            // returned in advance on transaction operations not on commit
            .map_err(Error::GroveDB)
    }
}
