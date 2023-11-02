use crate::error::Error;
use crate::execution::storage::{STORAGE_KEY, STORAGE_PATH};
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use drive::query::Element;

impl<C> Platform<C> {
    pub(super) fn store_execution_state_v0(
        &self,
        state: &PlatformState,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut ops = Vec::new();

        let element = Element::Item(state.serialize_to_bytes()?, None);

        self.drive
            .grove_insert(
                STORAGE_PATH.into(),
                STORAGE_KEY,
                element,
                Some(transaction),
                None,
                &mut ops,
                &platform_version.drive,
            )
            .map_err(|e| Error::Drive(e))
    }
}
