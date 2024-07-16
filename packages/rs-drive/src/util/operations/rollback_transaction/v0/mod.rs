use crate::drive::Drive;
use crate::error::Error;
use grovedb::Transaction;

impl Drive {
    /// Rolls back a transaction.
    pub(crate) fn rollback_transaction_v0(&self, transaction: &Transaction) -> Result<(), Error> {
        self.grove
            .rollback_transaction(transaction)
            .map_err(Error::GroveDB)
    }
}
