use crate::drive::platform_state::PLATFORM_STATE_KEY;
use crate::drive::Drive;
use crate::error::Error;
use grovedb::TransactionArg;

impl Drive {
    pub(super) fn store_platform_state_bytes_v0(
        &self,
        state_bytes: &[u8],
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        self.grove
            .put_aux(PLATFORM_STATE_KEY, state_bytes, None, transaction)
            .unwrap()
            .map_err(Error::GroveDB)
    }
}
