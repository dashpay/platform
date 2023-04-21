use dashcore::{Block, BlockHash, QuorumHash, Transaction, Txid};
use dashcore_rpc::dashcore_rpc_json::{
    Bip9SoftforkInfo, Bip9SoftforkStatus, ExtendedQuorumDetails, ExtendedQuorumListResult,
    GetBestChainLockResult, QuorumInfoResult, QuorumListResult, QuorumType,
};
use dashcore_rpc::json::{GetTransactionResult, MasternodeListDiffWithMasternodes};
use dashcore_rpc::{Auth, Client, Error, RpcApi};
use mockall::{automock, predicate::*};
use serde_json::Value;
use std::collections::HashMap;
use tenderdash_abci::proto::types::CoreChainLock;

/// Information returned by QuorumListExtended
pub type QuorumListExtendedInfo = HashMap<QuorumHash, ExtendedQuorumDetails>;

/// Core height must be of type u32 (Platform heights are u64)
pub type CoreHeight = u32;
/// Core RPC interface
#[automock]
pub trait CoreRPCLike {
    /// Get block hash by height
    fn get_block_hash(&self, height: CoreHeight) -> Result<BlockHash, Error>;

    /// Get block hash by height
    fn get_best_chain_lock(&self) -> Result<CoreChainLock, Error>;

    /// Get transaction
    fn get_transaction(&self, tx_id: &Txid) -> Result<Transaction, Error>;

    /// Get transaction
    fn get_transaction_extended_info(&self, tx_id: &Txid) -> Result<GetTransactionResult, Error>;

    /// Get block by hash
    fn get_fork_info(&self, name: &str) -> Result<Option<Bip9SoftforkInfo>, Error>;

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
    ) -> Result<ExtendedQuorumListResult, Error>;

    /// Get quorum information.
    ///
    /// See https://dashcore.readme.io/v19.0.0/docs/core-api-ref-remote-procedure-calls-evo#quorum-info
    fn get_quorum_info(
        &self,
        quorum_type: QuorumType,
        hash: &QuorumHash,
        include_secret_key_share: Option<bool>,
    ) -> Result<QuorumInfoResult, Error>;

    /// Get the difference in masternode list, return masternodes as diff elements
    fn get_protx_diff_with_masternodes(
        &self,
        base_block: u32,
        block: u32,
    ) -> Result<MasternodeListDiffWithMasternodes, Error>;

    // /// Get the detailed information about a deterministic masternode
    // fn get_protx_info(&self, protx_hash: &ProTxHash) -> Result<ProTxInfo, Error>;
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
        self.inner.get_block_hash(height)
    }

    fn get_best_chain_lock(&self) -> Result<CoreChainLock, Error> {
        let GetBestChainLockResult {
            blockhash,
            height,
            signature,
            known_block: _,
        } = self.inner.get_best_chain_lock()?;
        Ok(CoreChainLock {
            core_block_height: height,
            core_block_hash: blockhash.to_vec(),
            signature,
        })
    }

    fn get_transaction(&self, tx_id: &Txid) -> Result<Transaction, Error> {
        self.inner.get_raw_transaction(tx_id, None)
    }

    fn get_transaction_extended_info(&self, tx_id: &Txid) -> Result<GetTransactionResult, Error> {
        self.inner.get_transaction(tx_id, None)
    }

    fn get_fork_info(&self, name: &str) -> Result<Option<Bip9SoftforkInfo>, Error> {
        let blockchain_info = self.inner.get_blockchain_info()?;
        Ok(blockchain_info
            .bip9_softforks
            .get(name)
            .map(|info| info.clone()))
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
    ) -> Result<ExtendedQuorumListResult, Error> {
        self.inner.get_quorum_listextended(height.map(|i| i))
    }

    fn get_quorum_info(
        &self,
        quorum_type: QuorumType,
        hash: &QuorumHash,
        include_secret_key_share: Option<bool>,
    ) -> Result<QuorumInfoResult, Error> {
        self.inner
            .get_quorum_info(quorum_type, hash, include_secret_key_share)
    }

    fn get_protx_diff_with_masternodes(
        &self,
        _base_block: u32,
        _block: u32,
    ) -> Result<MasternodeListDiffWithMasternodes, Error> {
        // method does not yet exist in core
        todo!()
    }

    // fn get_protx_info(&self, protx_hash: &ProTxHash) -> Result<ProTxInfo, Error> {
    //     self.inner.get_protx_info(protx_hash)
    // }
}
