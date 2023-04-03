use crate::error::Error;
use dashcore::{Block, BlockHash};
use dashcore_rpc::dashcore_rpc_json::GetBestChainLockResult;
use dashcore_rpc::{Auth, Client, RpcApi};
use mockall::{automock, predicate::*};
use serde_json::Value;
use tenderdash_abci::proto::types::CoreChainLock;

/// Core RPC interface
#[automock]
pub trait CoreRPCLike {
    /// Get block hash by height
    fn get_block_hash(&self, height: u32) -> Result<BlockHash, Error>;

    /// Get block hash by height
    fn get_best_chain_lock(&self) -> Result<CoreChainLock, Error>;

    /// Get block by hash
    fn get_block(&self, block_hash: &BlockHash) -> Result<Block, Error>;

    /// Get block by hash in JSON format
    fn get_block_json(&self, block_hash: &BlockHash) -> Result<Value, Error>;
}

/// Default implementation of Dash Core RPC using DashCoreRPC client
pub struct DefaultCoreRPC {
    inner: Client,
}

impl DefaultCoreRPC {
    /// Create new instance
    pub fn open(url: &str, username: String, password: String) -> Result<Self, Error> {
        Ok(DefaultCoreRPC {
            inner: Client::new(url, Auth::UserPass(username, password))?,
        })
    }
}

impl CoreRPCLike for DefaultCoreRPC {
    fn get_block_hash(&self, height: u32) -> Result<BlockHash, Error> {
        self.inner.get_block_hash(height).map_err(Error::CoreRpc)
    }

    fn get_best_chain_lock(&self) -> Result<CoreChainLock, Error> {
        let GetBestChainLockResult {
            blockhash,
            height,
            signature,
            known_block,
        } = self.inner.get_best_chain_lock().map_err(Error::CoreRpc)?;
        Ok(CoreChainLock {
            core_block_height: height,
            core_block_hash: blockhash.to_vec(),
            signature,
        })
    }

    fn get_block(&self, block_hash: &BlockHash) -> Result<Block, Error> {
        self.inner.get_block(block_hash).map_err(Error::CoreRpc)
    }

    fn get_block_json(&self, block_hash: &BlockHash) -> Result<Value, Error> {
        self.inner
            .get_block_json(block_hash)
            .map_err(Error::CoreRpc)
    }
}
