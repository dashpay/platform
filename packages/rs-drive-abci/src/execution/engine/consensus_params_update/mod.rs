use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::engine::consensus_params_update::v0::consensus_params_update_v0;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::types::ConsensusParams;
mod v0;
pub(crate) fn consensus_params_update(
    original_platform_version: &PlatformVersion,
    new_platform_version: &PlatformVersion,
) -> Result<Option<ConsensusParams>, Error> {
    match new_platform_version
        .drive_abci
        .methods
        .engine
        .consensus_params_update
    {
        0 => Ok(consensus_params_update_v0(
            original_platform_version,
            new_platform_version,
        )),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "consensus_params_update".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}
