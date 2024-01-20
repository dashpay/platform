use crate::error::Error;
use crate::execution::storage::EXECUTION_STORAGE_STATE_KEY;
use crate::platform_types::platform_state::PlatformState;
use dpp::serialization::PlatformDeserializableFromVersionedStructure;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::query::TransactionArg;

pub(super) fn fetch_execution_state_v0(
    drive: &Drive,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Option<PlatformState>, Error> {
    let maybe_bytes = drive
        .grove
        .get_aux(EXECUTION_STORAGE_STATE_KEY, transaction)
        .unwrap()
        .map_err(Error::GroveDb)?;

    let Some(bytes) = maybe_bytes else {
        return Ok(None);
    };

    let execution_state = PlatformState::versioned_deserialize(&bytes, platform_version)?;

    Ok(Some(execution_state))
}
