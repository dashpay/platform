use crate::drive::execution_state::EXECUTION_STORAGE_STATE_KEY;
use crate::drive::Drive;
use crate::error::Error;
use dpp::serialization::PlatformSerializable;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn store_platform_state_bytes_v0(
        &self,
        state_bytes: &[u8],
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        self.grove
            .put_aux(EXECUTION_STORAGE_STATE_KEY, state_bytes, None, transaction)
            .unwrap()
            .map_err(Error::GroveDB)
    }
}
