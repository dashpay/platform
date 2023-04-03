use std::collections::HashMap;

use dashcore_rpc::dashcore::{Block, BlockHash};
use dashcore_rpc::{
    dashcore_rpc_json::{
        ExtendedQuorumDetails, QuorumHash, QuorumInfoResult, QuorumListResult, QuorumType,
    },
    Auth, Client, Error, RpcApi,
};

#[cfg(feature = "fixtures-and-mocks")]
use mockall::{automock, predicate::*};
use serde_json::Value;

/// Information returned by QuorumListExtended
pub type QuorumListExtendedInfo = HashMap<QuorumHash, ExtendedQuorumDetails>;

/// Core height must be of type u32 (Platform heights are u64)
pub type CoreHeight = u32;
/// Core RPC interface
#[cfg_attr(feature = "fixtures-and-mocks", automock)]
pub trait CoreRPCLike {
    /// Get block hash by height
    fn get_block_hash(&self, height: CoreHeight) -> Result<BlockHash, Error>;

    /// Get block by hash
    fn get_block(&self, block_hash: &BlockHash) -> Result<Block, Error>;

    /// Get block by hash in JSON format
    fn get_block_json(&self, block_hash: &BlockHash) -> Result<Value, Error>;

    /// Get list of quorums at a given height.
    ///
    /// See https://dashcore.readme.io/v19.0.0/docs/core-api-ref-remote-procedure-calls-evo#quorum-listextended
    fn get_quorum_listextended(
        &self,
        height: Option<CoreHeight>,
    ) -> Result<QuorumListResult<QuorumListExtendedInfo>, Error>;

    /// Get quorum information.
    ///
    /// See https://dashcore.readme.io/v19.0.0/docs/core-api-ref-remote-procedure-calls-evo#quorum-info
    fn get_quorum_info(
        &self,
        quorum_type: QuorumType,
        hash: &QuorumHash,
        o: Option<bool>,
    ) -> Result<QuorumInfoResult, Error>;
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
    fn get_block_hash(&self, height: CoreHeight) -> Result<BlockHash, Error> {
        self.inner.get_block_hash(height)
    }

    fn get_block(&self, block_hash: &BlockHash) -> Result<Block, Error> {
        self.inner.get_block(block_hash)
    }

    fn get_block_json(&self, block_hash: &BlockHash) -> Result<Value, Error> {
        self.inner.get_block_json(block_hash)
    }
    fn get_quorum_listextended(
        &self,
        height: Option<CoreHeight>,
    ) -> Result<QuorumListResult<QuorumListExtendedInfo>, Error> {
        self.inner.get_quorum_listextended(height.map(|i| i as i64))
    }

    fn get_quorum_info(
        &self,
        quorum_type: QuorumType,
        quorum_hash: &QuorumHash,
        include_sk_share: Option<bool>,
    ) -> Result<QuorumInfoResult, Error> {
        self.inner
            .get_quorum_info(quorum_type, quorum_hash, include_sk_share)
    }
}
