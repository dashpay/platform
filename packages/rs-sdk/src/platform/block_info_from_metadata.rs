use dapi_grpc::platform::v0::ResponseMetadata;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::MAX_EPOCH;
use drive::error::proof::ProofError;
use crate::Error;

pub fn block_info_from_metadata(response_metadata: &ResponseMetadata) -> Result<BlockInfo, Error> {
    if  response_metadata.epoch > MAX_EPOCH as u32 {
        return Err(drive::error::Error::Proof(ProofError::InvalidMetadata(format!("platform returned an epoch {} that was higher that maximum of a 16 bit integer", response_metadata.epoch))).into());
    }

    Ok(BlockInfo {
        time_ms: response_metadata.time_ms,
        height: response_metadata.height,
        core_height: response_metadata.core_chain_locked_height,
        epoch: (response_metadata.epoch as u16).try_into()?,
    })
}
