use crate::error::Error;
use crate::execution::storage::EXECUTION_STORAGE_STATE_KEY;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::serialization::PlatformSerializable;
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    pub(super) fn store_execution_state_v0(
        &self,
        state: &PlatformState,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        self.drive
            .grove
            .put_aux(
                EXECUTION_STORAGE_STATE_KEY,
                &state.serialize_to_bytes()?,
                None,
                Some(transaction),
            )
            .unwrap()
            .map_err(Error::GroveDb)
    }
}
