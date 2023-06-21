use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use drive::grovedb::Transaction;
use tenderdash_abci::proto::abci::{RequestInitChain, ResponseInitChain};

mod v0;

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
        //todo: use protocol version to decide
        self.init_chain_v0(request, transaction)
    }
}
