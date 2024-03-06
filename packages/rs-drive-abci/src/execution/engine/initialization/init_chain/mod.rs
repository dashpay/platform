mod v0;

use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use crate::error::execution::ExecutionError;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use dpp::version::PlatformVersion;
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
        let protocol_version = self.config.initial_protocol_version;
        let platform_version = PlatformVersion::get(protocol_version)?;

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
