use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::storage::EXECUTION_STORAGE_PATH;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::error::drive::DriveError;
use drive::grovedb::Element;

pub(super) const EXECUTION_STORAGE_PLATFORM_VERSION_KEY: &[u8; 1] = b"V";

/// Fetches the current execution protocol version from the drive
/// This method can't be versioned since there is no knowledge about versions before it called
/// but fallbacks to support all changes in this method must be implemented
pub fn fetch_current_protocol_version(drive: &Drive) -> Result<Option<u32>, Error> {
    let maybe_element = drive
        .grove
        .get_raw_optional(
            (&EXECUTION_STORAGE_PATH).into(),
            EXECUTION_STORAGE_PLATFORM_VERSION_KEY,
            None,
        )
        .unwrap()
        .map_err(|e| Error::Drive(drive::error::Error::GroveDB(e)))?;

    let Some(element) = maybe_element else {
        return Ok(None);
    };

    let Element::Item(bytes, _) = element else {
        return Err(Error::Execution(ExecutionError::CorruptedCachedState(
            "execution state should be stored as an element item",
        )));
    };

    let protocol_version = u32::from_be_bytes(bytes.as_slice().try_into().map_err(|_| {
        drive::error::Error::Drive(DriveError::CorruptedSerialization(String::from(
            "protocol version length item have an invalid length",
        )))
    })?);

    Ok(Some(protocol_version))
}
