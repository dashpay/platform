use crate::Error;
use dapi_grpc::platform::v0::ResponseMetadata;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::MAX_EPOCH;
use drive::error::proof::ProofError;

/// Constructs a `BlockInfo` structure from the provided response metadata. This function
/// translates metadata received from a platform response into a format that is specific to the
/// application's needs, particularly focusing on block-related information. It ensures that
/// the epoch value from the metadata does not exceed the maximum allowed for a 16-bit integer,
/// as this is a constraint for the `BlockInfo` structure.
///
/// # Parameters
/// - `response_metadata`: A reference to `ResponseMetadata` obtained from a platform response.
/// This metadata includes various block-related information such as time in milliseconds,
/// height, core chain locked height, and epoch.
///
/// # Returns
/// If successful, returns `Ok(BlockInfo)` where `BlockInfo` contains:
/// - `time_ms`: The timestamp of the block in milliseconds.
/// - `height`: The height of the block.
/// - `core_height`: The core chain locked height, indicating the height of the block in the core blockchain that is considered final and securely linked to this block.
/// - `epoch`: The epoch number, converted to a 16-bit integer.
///
/// # Errors
/// Returns an error if:
/// - The `epoch` value in the response metadata exceeds the maximum value that can be represented by a 16-bit integer. This is considered a data validity error as it indicates the platform returned an unexpectedly high epoch number.
///
/// The function encapsulates errors into the application's own `Error` type, providing a unified interface for error handling across the application.
pub fn block_info_from_metadata(response_metadata: &ResponseMetadata) -> Result<BlockInfo, Error> {
    if response_metadata.epoch > MAX_EPOCH as u32 {
        return Err(
            drive::error::Error::Proof(ProofError::InvalidMetadata(format!(
                "platform returned an epoch {} that was higher that maximum of a 16 bit integer",
                response_metadata.epoch
            )))
            .into(),
        );
    }

    Ok(BlockInfo {
        time_ms: response_metadata.time_ms,
        height: response_metadata.height,
        core_height: response_metadata.core_chain_locked_height,
        epoch: (response_metadata.epoch as u16).try_into()?,
    })
}
