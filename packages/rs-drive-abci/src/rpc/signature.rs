use super::core::CORE_RPC_INVALID_ADDRESS_OR_KEY;
use super::core::CORE_RPC_INVALID_PARAMETER;
use super::core::CORE_RPC_PARSE_ERROR;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::InstantLock;

/// An entity with a signature verifiable by Core RPC
pub trait CoreSignatureVerification {
    /// Verify signature with Core RPC
    fn verify_signature<C: CoreRPCLike>(
        &self,
        core_rpc: &C,
        core_chain_locked_height: u32,
    ) -> Result<bool, Error>;
}

impl CoreSignatureVerification for InstantLock {
    fn verify_signature<C: CoreRPCLike>(
        &self,
        core_rpc: &C,
        core_chain_locked_height: u32,
    ) -> Result<bool, Error> {
        match core_rpc.verify_instant_lock(self, Some(core_chain_locked_height)) {
            Ok(result) => Ok(result),
            // Consider signature is invalid in case if instant lock data format is wrong for some reason
            Err(dpp::dashcore_rpc::Error::JsonRpc(
                dpp::dashcore_rpc::jsonrpc::error::Error::Rpc(
                    dpp::dashcore_rpc::jsonrpc::error::RpcError {
                        code:
                            CORE_RPC_PARSE_ERROR
                            | CORE_RPC_INVALID_ADDRESS_OR_KEY
                            | CORE_RPC_INVALID_PARAMETER,
                        ..
                    },
                ),
            )) => Ok(false),
            Err(e) => Err(Error::Execution(ExecutionError::DashCoreBadResponseError(
                format!("can't verify instant asset lock proof signature with core: {e}",),
            ))),
        }
    }
}
