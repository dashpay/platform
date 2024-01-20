use crate::error::Error;
use crate::execution::storage::{EXECUTION_STORAGE_PATH, EXECUTION_STORAGE_STATE_KEY};
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use drive::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use drive::drive::batch::GroveDbOpBatch;
use drive::grovedb::Transaction;
use drive::query::Element;

impl<C> Platform<C> {
    pub(super) fn store_execution_state_v0(
        &self,
        state: &PlatformState,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut batch = GroveDbOpBatch::new();

        let path: Vec<Vec<u8>> = EXECUTION_STORAGE_PATH
            .iter()
            .map(|byte_array| byte_array.to_vec())
            .collect();

        let state_element = Element::Item(state.serialize_to_bytes()?, None);

        batch.add_insert(path, EXECUTION_STORAGE_STATE_KEY.to_vec(), state_element);

        self.drive
            .grove_apply_batch(batch, false, Some(transaction), &platform_version.drive)
            .map_err(Error::Drive)
    }
}
