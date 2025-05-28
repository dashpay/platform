use dashcore_rpc::dashcore_rpc_json::{
    AssetUnlockStatusResult, ExtendedQuorumListResult, GetChainTipsResult, GetRawTransactionResult,
    MasternodeListDiff, MnSyncStatus, QuorumInfoResult, QuorumType, SoftforkInfo,
};
use dashcore_rpc::json::SoftforkType;
use dashcore_rpc::Error;
use dpp::dashcore::bls_sig_utils::BLSSignature;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{
    Block, BlockHash, ChainLock, Header, InstantLock, QuorumHash, Transaction, Txid,
};
use dpp::prelude::TimestampMillis;
use drive_abci::rpc::core::{CoreHeight, CoreRPCLike};
use serde_json::Value;

fn load_quorum_info_from_file(file_name: &str) -> Result<QuorumInfoResult, Error> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/production_block_execution_tests/mainnet_genesis_test_data")
        .join(file_name);

    let file = std::fs::File::open(&path).map_err(|e| {
        Error::UnexpectedStructure(format!(
            "failed to open quorum info file: {} at path {}",
            e,
            path.to_string_lossy()
        ))
    })?;

    serde_json::from_reader(file).map_err(|e| {
        Error::UnexpectedStructure(format!(
            "failed to deserialize quorum info: {} for {}",
            e, file_name
        ))
    })
}

#[derive(Default)]
pub struct MainnetForTestsCoreRpcLike;
impl CoreRPCLike for MainnetForTestsCoreRpcLike {
    fn get_block_hash(&self, _height: CoreHeight) -> Result<BlockHash, Error> {
        Err(Error::UnexpectedStructure(
            "get_block_hash not implemented".to_string(),
        ))
    }

    fn get_block_header(&self, _block_hash: &BlockHash) -> Result<Header, Error> {
        Err(Error::UnexpectedStructure(
            "get_block_header not implemented".to_string(),
        ))
    }

    fn get_block_time_from_height(&self, height: CoreHeight) -> Result<TimestampMillis, Error> {
        match height {
            2128896 => Ok(1724795532),
            _ => Err(Error::UnexpectedStructure(format!(
                "no block time for height {}",
                height
            ))),
        }
    }

    fn get_best_chain_lock(&self) -> Result<ChainLock, Error> {
        Ok(ChainLock {
            block_height: 2_500_000,
            block_hash: BlockHash::all_zeros(),
            signature: BLSSignature::from([0; 96]),
        })
    }

    fn submit_chain_lock(&self, _chain_lock: &ChainLock) -> Result<u32, Error> {
        Ok(2_500_000)
    }

    fn get_transaction(&self, _tx_id: &Txid) -> Result<Transaction, Error> {
        Err(Error::UnexpectedStructure(
            "get_transaction not implemented".to_string(),
        ))
    }

    fn get_asset_unlock_statuses(
        &self,
        _indices: &[u64],
        _core_chain_locked_height: u32,
    ) -> Result<Vec<AssetUnlockStatusResult>, Error> {
        Err(Error::UnexpectedStructure(
            "get_asset_unlock_statuses not implemented".to_string(),
        ))
    }

    fn get_transaction_extended_info(
        &self,
        _tx_id: &Txid,
    ) -> Result<GetRawTransactionResult, Error> {
        Err(Error::UnexpectedStructure(
            "get_transaction_extended_info not implemented".to_string(),
        ))
    }

    fn get_fork_info(&self, name: &str) -> Result<Option<SoftforkInfo>, Error> {
        match name {
            "mn_rr" => Ok(Some(SoftforkInfo {
                softfork_type: SoftforkType::Bip9,
                active: true,
                height: Some(2128896),
                bip9: None,
            })),
            other => Err(Error::UnexpectedStructure(format!(
                "no fork info for {}",
                other
            ))),
        }
    }

    fn get_block(&self, _block_hash: &BlockHash) -> Result<Block, Error> {
        Err(Error::UnexpectedStructure(
            "get_block not implemented".to_string(),
        ))
    }

    fn get_block_json(&self, _block_hash: &BlockHash) -> Result<Value, Error> {
        Err(Error::UnexpectedStructure(
            "get_block_json not implemented".to_string(),
        ))
    }

    fn get_chain_tips(&self) -> Result<GetChainTipsResult, Error> {
        Err(Error::UnexpectedStructure(
            "get_chain_tips not implemented".to_string(),
        ))
    }

    fn get_quorum_listextended(
        &self,
        height: Option<CoreHeight>,
    ) -> Result<ExtendedQuorumListResult, Error> {
        match height {
            Some(2128896) => {
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/production_block_execution_tests/mainnet_genesis_test_data/quorum_list_extended_2128896.json");
                let file = std::fs::File::open(path.clone()).map_err(|e| {
                    Error::UnexpectedStructure(format!(
                        "failed to open quorum_list_extended file: {} at path {}",
                        e,
                        path.to_string_lossy()
                    ))
                })?;

                let quorum_list_result: ExtendedQuorumListResult = serde_json::from_reader(file)
                    .map_err(|e| {
                        Error::UnexpectedStructure(format!(
                            "failed to deserialize quorum_list_extended: {}",
                            e
                        ))
                    })?;

                Ok(quorum_list_result)
            }
            Some(2132092) => {
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/production_block_execution_tests/mainnet_genesis_test_data/quorum_list_extended_2132092.json");
                let file = std::fs::File::open(path.clone()).map_err(|e| {
                    Error::UnexpectedStructure(format!(
                        "failed to open quorum_list_extended file: {} at path {}",
                        e,
                        path.to_string_lossy()
                    ))
                })?;

                let quorum_list_result: ExtendedQuorumListResult = serde_json::from_reader(file)
                    .map_err(|e| {
                        Error::UnexpectedStructure(format!(
                            "failed to deserialize quorum_list_extended: {}",
                            e
                        ))
                    })?;

                Ok(quorum_list_result)
            }
            height => Err(Error::UnexpectedStructure(format!(
                "no quorum list extended known at height {:?}",
                height
            ))),
        }
    }

    fn get_quorum_info(
        &self,
        quorum_type: QuorumType,
        hash: &QuorumHash,
        _include_secret_key_share: Option<bool>,
    ) -> Result<QuorumInfoResult, Error> {
        match (quorum_type, hex::encode(hash.to_raw_hash()).as_str()) {
            (QuorumType::Llmq100_67, hash) => {
                load_quorum_info_from_file(format!("quorum_info_100_67_{}.json", hash).as_str())
            }
            (QuorumType::Llmq400_60, hash) => {
                load_quorum_info_from_file(format!("quorum_info_400_60_{}.json", hash).as_str())
            }
            (QuorumType::Llmq60_75, hash) => {
                load_quorum_info_from_file(format!("quorum_info_60_75_{}.json", hash).as_str())
            }

            (_, _) => Err(Error::UnexpectedStructure(format!(
                "no quorum info known for quorum hash {} of type {}",
                hash, quorum_type
            ))),
        }
    }

    fn get_protx_diff_with_masternodes(
        &self,
        base_block: Option<u32>,
        block: u32,
    ) -> Result<MasternodeListDiff, Error> {
        match (base_block, block) {
            (None, 2128896) => {
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/production_block_execution_tests/mainnet_genesis_test_data/protx_listdiff_1_2128896.json");
                let file = std::fs::File::open(path.clone()).map_err(|e| {
                    Error::UnexpectedStructure(format!(
                        "failed to open protx_diff file: {} at path {}",
                        e,
                        path.to_string_lossy()
                    ))
                })?;

                let diff: MasternodeListDiff = serde_json::from_reader(file).map_err(|e| {
                    Error::UnexpectedStructure(format!("failed to deserialize protx_diff: {}", e))
                })?;

                Ok(diff)
            }
            (Some(2128896), 2132092) => {
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/production_block_execution_tests/mainnet_genesis_test_data/protx_listdiff_2128896_2132092.json");
                let file = std::fs::File::open(path.clone()).map_err(|e| {
                    Error::UnexpectedStructure(format!(
                        "failed to open protx_diff file: {} at path {}",
                        e,
                        path.to_string_lossy()
                    ))
                })?;

                let diff: MasternodeListDiff = serde_json::from_reader(file).map_err(|e| {
                    Error::UnexpectedStructure(format!("failed to deserialize protx_diff: {}", e))
                })?;

                Ok(diff)
            }
            (base, block) => Err(Error::UnexpectedStructure(format!(
                "no pro_tx_diff known from {:?} to {}",
                base, block
            ))),
        }
    }

    fn verify_instant_lock(
        &self,
        _instant_lock: &InstantLock,
        _max_height: Option<u32>,
    ) -> Result<bool, Error> {
        Err(Error::UnexpectedStructure(
            "verify_instant_lock not implemented".to_string(),
        ))
    }

    fn verify_chain_lock(&self, _chain_lock: &ChainLock) -> Result<bool, Error> {
        Err(Error::UnexpectedStructure(
            "verify_chain_lock not implemented".to_string(),
        ))
    }

    fn masternode_sync_status(&self) -> Result<MnSyncStatus, Error> {
        Err(Error::UnexpectedStructure(
            "masternode_sync_status not implemented".to_string(),
        ))
    }

    fn send_raw_transaction(&self, _transaction: &[u8]) -> Result<Txid, Error> {
        Err(Error::UnexpectedStructure(
            "send_raw_transaction not implemented".to_string(),
        ))
    }
}
