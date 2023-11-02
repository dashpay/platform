use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::storage::{STORAGE_KEY, STORAGE_PATH};
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::serialization::PlatformDeserializable;
use dpp::version::PlatformVersion;
use drive::drive::grove_operations::QueryType;
use drive::query::{Element, TransactionArg};

impl<C> Platform<C> {
    pub(super) fn fetch_execution_state_v0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<PlatformState>, Error> {
        let mut ops = Vec::new();

        let maybe_element = self
            .drive
            .grove_get(
                STORAGE_PATH.into(),
                STORAGE_KEY,
                QueryType::StatefulQuery,
                transaction,
                &mut ops,
                &platform_version.drive,
            )
            .map_err(|e| Error::Drive(e))?;

        let Some(element) = maybe_element else {
            return Ok(None);
        };

        let Element::Item(bytes, _) = element else {
            return Err(Error::Execution(ExecutionError::CorruptedCachedState(
                "execution state should be stored as an element item",
            )));
        };

        let execution_state = PlatformState::deserialize_from_bytes(&bytes)?;

        Ok(Some(execution_state))
    }
}
