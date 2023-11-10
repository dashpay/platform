use crate::error::Error;
use crate::execution::storage::protocol_version::EXECUTION_STORAGE_PLATFORM_VERSION_KEY;
use crate::execution::storage::{EXECUTION_STORAGE_PATH, EXECUTION_STORAGE_STATE_KEY};
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
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

        let protocol_version_element = Element::Item(
            state
                .current_protocol_version_in_consensus()
                .to_be_bytes()
                .to_vec(),
            None,
        );

        batch.add_insert(
            path.clone(),
            EXECUTION_STORAGE_PLATFORM_VERSION_KEY.to_vec(),
            protocol_version_element,
        );

        let state_element = Element::Item(state.serialize_to_bytes()?, None);

        batch.add_insert(path, EXECUTION_STORAGE_STATE_KEY.to_vec(), state_element);

        self.drive
            .grove_apply_batch(batch, false, Some(transaction), &platform_version.drive)
            .map_err(Error::Drive)
    }
}
