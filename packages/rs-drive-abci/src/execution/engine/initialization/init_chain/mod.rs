mod v0;

use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use crate::abci::AbciError;
use crate::error::execution::ExecutionError;
use dpp::version::PlatformVersion;
use dpp::version::ProtocolVersion;
use drive::grovedb::Transaction;
use tenderdash_abci::proto::abci::{RequestInitChain, ResponseInitChain};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Initialize the chain
    pub fn init_chain(
        &self,
        request: RequestInitChain,
        transaction: &Transaction,
    ) -> Result<ResponseInitChain, Error> {
        // We don't have platform state at this point, so we should
        // use initial protocol version from genesis
        let consensus_params = request
            .consensus_params
            .as_ref()
            .ok_or(AbciError::BadRequest(
                "consensus params are required in init chain".to_string(),
            ))?;

        let tenderdash_abci::proto::types::VersionParams {
            app_version: protocol_version,
            ..
        } = consensus_params
            .version
            .as_ref()
            .ok_or(AbciError::BadRequest(
                "consensus params version is required in init chain".to_string(),
            ))?;

        let platform_version = if *protocol_version == 0 {
            // Protocol version is not set.
            // We are starting the chain with the desired version
            PlatformVersion::desired()
        } else {
            // Use the version from the genesis
            PlatformVersion::get(*protocol_version as ProtocolVersion)?
        };

        match platform_version.drive_abci.methods.engine.init_chain {
            0 => self.init_chain_v0(request, transaction, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "init_chain".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
