use crate::drive::execution_state::EXECUTION_STORAGE_STATE_KEY;
use crate::drive::Drive;
use crate::error::Error;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn fetch_execution_state_bytes_v0(
        &self,
        transaction: TransactionArg,
    ) -> Result<Option<Vec<u8>>, Error> {
        self.grove
            .get_aux(EXECUTION_STORAGE_STATE_KEY, transaction)
            .unwrap()
            .map_err(Error::GroveDB)
    }
}
