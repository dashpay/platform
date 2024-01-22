use crate::error::Error;
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
        .fetch_execution_state_bytes(transaction, platform_version)
        .map_err(Error::Drive)?;

    let Some(bytes) = maybe_bytes else {
        return Ok(None);
    };

    let execution_state = PlatformState::versioned_deserialize(&bytes, platform_version)?;

    Ok(Some(execution_state))
}
