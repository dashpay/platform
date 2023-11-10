use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::storage::{EXECUTION_STORAGE_PATH, EXECUTION_STORAGE_STATE_KEY};
use crate::platform_types::platform_state::PlatformState;
use dpp::serialization::{PlatformDeserializable, PlatformDeserializableFromVersionedStructure};
use dpp::version::PlatformVersion;
use drive::drive::grove_operations::QueryType;
use drive::drive::Drive;
use drive::error::drive::DriveError;
use drive::query::{Element, TransactionArg};

pub(super) fn fetch_execution_state_v0(
    drive: &Drive,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Option<PlatformState>, Error> {
    let mut ops = Vec::new();

    let maybe_element = drive
        .grove_get(
            (&EXECUTION_STORAGE_PATH).into(),
            EXECUTION_STORAGE_STATE_KEY,
            QueryType::StatefulQuery,
            transaction,
            &mut ops,
            &platform_version.drive,
        )
        .map_err(Error::Drive)?;

    let Some(element) = maybe_element else {
        return Ok(None);
    };

    let Element::Item(bytes, _) = element else {
        return Err(Error::Execution(ExecutionError::CorruptedCachedState(
            "execution state should be stored as an element item",
        )));
    };

    let execution_state = PlatformState::versioned_deserialize(&bytes, platform_version)?;

    Ok(Some(execution_state))
}
