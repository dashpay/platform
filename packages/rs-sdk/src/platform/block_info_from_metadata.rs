//! SDK-facing wrapper over the transport-free block-info helper.
//!
//! The implementation lives in
//! [`dash_platform_queries::block_info_from_metadata`] so embedders that skip
//! `dash-sdk` can use it. This module keeps the historical `dash-sdk`
//! signature — `Result<BlockInfo, dash_sdk::Error>` — because a `From`
//! conversion on the error type does not preserve explicit return types,
//! direct variant matching, or function-pointer signatures for existing
//! callers.

use crate::error::Error;
use dapi_grpc::platform::v0::ResponseMetadata;
use dpp::block::block_info::BlockInfo;

/// Constructs a [`BlockInfo`] from the provided response metadata.
///
/// Thin forwarder over
/// [`dash_platform_queries::block_info_from_metadata::block_info_from_metadata`];
/// see there for the full contract. The only difference is the error type,
/// which stays [`crate::Error`] for source compatibility.
///
/// # Errors
///
/// Returns an error if the metadata's `epoch` exceeds
/// [`MAX_EPOCH`](dpp::block::epoch::MAX_EPOCH), which means Platform returned
/// an unexpectedly high epoch number.
pub fn block_info_from_metadata(response_metadata: &ResponseMetadata) -> Result<BlockInfo, Error> {
    dash_platform_queries::block_info_from_metadata::block_info_from_metadata(response_metadata)
        .map_err(Error::from)
}
