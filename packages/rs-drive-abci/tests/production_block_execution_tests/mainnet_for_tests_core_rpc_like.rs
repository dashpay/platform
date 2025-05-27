use dashcore_rpc::dashcore_rpc_json::{AssetUnlockStatusResult, ExtendedQuorumListResult, GetChainTipsResult, GetRawTransactionResult, MasternodeListDiff, MnSyncStatus, QuorumInfoResult, QuorumType, SoftforkInfo};
use dashcore_rpc::Error;
use dashcore_rpc::json::SoftforkType;
use serde_json::Value;
use dpp::dashcore::{Block, BlockHash, ChainLock, Header, InstantLock, QuorumHash, Transaction, Txid};
use dpp::dashcore::bls_sig_utils::BLSSignature;
use dpp::dashcore::hashes::Hash;
use dpp::prelude::TimestampMillis;
use drive_abci::rpc::core::{CoreHeight, CoreRPCLike};

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
        Error::UnexpectedStructure(format!("failed to deserialize quorum info: {}", e))
    })
}

#[derive(Default)]
pub struct MainnetForTestsCoreRpcLike;
impl CoreRPCLike for MainnetForTestsCoreRpcLike {
    fn get_block_hash(&self, _height: CoreHeight) -> Result<BlockHash, Error> {
        Err(Error::UnexpectedStructure("get_block_hash not implemented".to_string()))
    }

    fn get_block_header(&self, _block_hash: &BlockHash) -> Result<Header, Error> {
        Err(Error::UnexpectedStructure("get_block_header not implemented".to_string()))
    }

    fn get_block_time_from_height(&self, height: CoreHeight) -> Result<TimestampMillis, Error> {
        match height {
            2128896 => Ok(1724795532),
            _ => Err(Error::UnexpectedStructure(format!("no block time for height {}", height)))
        }
    }

    fn get_best_chain_lock(&self) -> Result<ChainLock, Error> {
        Ok(ChainLock {
            block_height: 2_500_000,
            block_hash: BlockHash::all_zeros(),
            signature: BLSSignature::from([0;96]),
        })
    }

    fn submit_chain_lock(&self, _chain_lock: &ChainLock) -> Result<u32, Error> {
        Err(Error::UnexpectedStructure("submit_chain_lock not implemented".to_string()))
    }

    fn get_transaction(&self, _tx_id: &Txid) -> Result<Transaction, Error> {
        Err(Error::UnexpectedStructure("get_transaction not implemented".to_string()))
    }

    fn get_asset_unlock_statuses(&self, _indices: &[u64], _core_chain_locked_height: u32) -> Result<Vec<AssetUnlockStatusResult>, Error> {
        Err(Error::UnexpectedStructure("get_asset_unlock_statuses not implemented".to_string()))
    }

    fn get_transaction_extended_info(&self, _tx_id: &Txid) -> Result<GetRawTransactionResult, Error> {
        Err(Error::UnexpectedStructure("get_transaction_extended_info not implemented".to_string()))
    }

    fn get_fork_info(&self, name: &str) -> Result<Option<SoftforkInfo>, Error> {
        match name {
            "mn_rr" => Ok(Some(SoftforkInfo {
                softfork_type: SoftforkType::Bip9,
                active: true,
                height: Some(2128896),
                bip9: None,
            })),
            other => Err(Error::UnexpectedStructure(format!("no fork info for {}", other)))
        }
    }

    fn get_block(&self, _block_hash: &BlockHash) -> Result<Block, Error> {
        Err(Error::UnexpectedStructure("get_block not implemented".to_string()))
    }

    fn get_block_json(&self, _block_hash: &BlockHash) -> Result<Value, Error> {
        Err(Error::UnexpectedStructure("get_block_json not implemented".to_string()))
    }

    fn get_chain_tips(&self) -> Result<GetChainTipsResult, Error> {
        Err(Error::UnexpectedStructure("get_chain_tips not implemented".to_string()))
    }

    fn get_quorum_listextended(&self, height: Option<CoreHeight>) -> Result<ExtendedQuorumListResult, Error> {
        match height {
            (Some(2128896)) => {
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/production_block_execution_tests/mainnet_genesis_test_data/quorum_list_extended_2128896.json");
                let file = std::fs::File::open(path.clone()).map_err(|e| {
                    Error::UnexpectedStructure(format!("failed to open quorum_list_extended file: {} at path {}", e, path.to_string_lossy()))
                })?;

                let quorum_list_result: ExtendedQuorumListResult = serde_json::from_reader(file).map_err(|e| {
                    Error::UnexpectedStructure(format!("failed to deserialize quorum_list_extended: {}", e))
                })?;

                Ok(quorum_list_result)
            }
            height => Err(Error::UnexpectedStructure(format!("no quorum list extended known at height {:?}", height)))
        }
    }

    fn get_quorum_info(&self, quorum_type: QuorumType, hash: &QuorumHash, _include_secret_key_share: Option<bool>) -> Result<QuorumInfoResult, Error> {
        match (quorum_type, hex::encode(hash.to_raw_hash()).as_str()) {
            (
                QuorumType::Llmq100_67,
                "000000000000000124fff6bc8b8b14e451c208115d2cfdc54106dcd71de98b59",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_000000000000000124fff6bc8b8b14e451c208115d2cfdc54106dcd71de98b59.json",
            ),

            (
                QuorumType::Llmq100_67,
                "0000000000000004d4b57d50a0f794151128ac79223d89b75fa36d9e41188bb0",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_0000000000000004d4b57d50a0f794151128ac79223d89b75fa36d9e41188bb0.json",
            ),
            (
                QuorumType::Llmq100_67,
                "0000000000000007da0261a7816493c4a4dfd7ddd6e7d82d00d8d0001d2f0c28",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_0000000000000007da0261a7816493c4a4dfd7ddd6e7d82d00d8d0001d2f0c28.json",
            ),
            (
                QuorumType::Llmq100_67,
                "0000000000000008fe479a2b20199bc361efff6ae14fa7c2f88366c19969e2af",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_0000000000000008fe479a2b20199bc361efff6ae14fa7c2f88366c19969e2af.json",
            ),
            (
                QuorumType::Llmq100_67,
                "000000000000000b4b95800801df4cea2fdb94189e53d725b765d2ea8f572f41",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_000000000000000b4b95800801df4cea2fdb94189e53d725b765d2ea8f572f41.json",
            ),
            (
                QuorumType::Llmq100_67,
                "000000000000000c061849709b4d26313ba203931c5afec017792334a912351e",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_000000000000000c061849709b4d26313ba203931c5afec017792334a912351e.json",
            ),
            (
                QuorumType::Llmq100_67,
                "000000000000000c8a95dc583ae769e6f75b2bd4c21b0cd4e5c61cda49b9843e",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_000000000000000c8a95dc583ae769e6f75b2bd4c21b0cd4e5c61cda49b9843e.json",
            ),
            (
                QuorumType::Llmq100_67,
                "000000000000000d1e93e70e0e6cc0c8536b83e343d899d555b15ebd1b4ac6d9",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_000000000000000d1e93e70e0e6cc0c8536b83e343d899d555b15ebd1b4ac6d9.json",
            ),
            (
                QuorumType::Llmq100_67,
                "000000000000000f1c612e9294ec71e156a1c61ea85483d6d5809cfd7e84b305",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_000000000000000f1c612e9294ec71e156a1c61ea85483d6d5809cfd7e84b305.json",
            ),
            (
                QuorumType::Llmq100_67,
                "00000000000000105399e8b0a97e71e9e0d12930e467774edf47f32dbaad96c9",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_00000000000000105399e8b0a97e71e9e0d12930e467774edf47f32dbaad96c9.json",
            ),
            (
                QuorumType::Llmq100_67,
                "00000000000000105f2d1ceda3c63d2b677a227d7ed77c5bad3776725cad0002",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_00000000000000105f2d1ceda3c63d2b677a227d7ed77c5bad3776725cad0002.json",
            ),
            (
                QuorumType::Llmq100_67,
                "0000000000000012ca2e8d01db451d0390ef172409ac86aa249c925956ac13d1",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_0000000000000012ca2e8d01db451d0390ef172409ac86aa249c925956ac13d1.json",
            ),
            (
                QuorumType::Llmq100_67,
                "00000000000000185e2056a59b5c9b593f0e8b7eff113dfdf99bd02be67287fb",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_00000000000000185e2056a59b5c9b593f0e8b7eff113dfdf99bd02be67287fb.json",
            ),
            (
                QuorumType::Llmq100_67,
                "00000000000000190428bba9fe36cf87ebe06afe5793c67d23c2741c8eed4b10",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_00000000000000190428bba9fe36cf87ebe06afe5793c67d23c2741c8eed4b10.json",
            ),
            (
                QuorumType::Llmq100_67,
                "000000000000001a795bbc6780540fb57d6f34b74e2a423b6d29d3fdfa3720e4",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_000000000000001a795bbc6780540fb57d6f34b74e2a423b6d29d3fdfa3720e4.json",
            ),
            (
                QuorumType::Llmq100_67,
                "000000000000001fec7152c0bbf6db912fd05bcf359d427625e1bb744700fbbe",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_000000000000001fec7152c0bbf6db912fd05bcf359d427625e1bb744700fbbe.json",
            ),
            (
                QuorumType::Llmq100_67,
                "0000000000000020c96803f4306b1ae861a7f980cad907f89d01f803373c4dc0",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_0000000000000020c96803f4306b1ae861a7f980cad907f89d01f803373c4dc0.json",
            ),
            (
                QuorumType::Llmq100_67,
                "000000000000002124827387934955dc34db0f6ecb9fb1143518466deb33538e",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_000000000000002124827387934955dc34db0f6ecb9fb1143518466deb33538e.json",
            ),
            (
                QuorumType::Llmq100_67,
                "00000000000000228a49a7526c7b259b3ef34b1830bbc81bc94fb0e44643b90d",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_00000000000000228a49a7526c7b259b3ef34b1830bbc81bc94fb0e44643b90d.json",
            ),
            (
                QuorumType::Llmq100_67,
                "000000000000002435bc2410f6bcd48b39ed1d139dfa496557080e7821302eed",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_000000000000002435bc2410f6bcd48b39ed1d139dfa496557080e7821302eed.json",
            ),
            (
                QuorumType::Llmq100_67,
                "0000000000000025ebf2166f97012aac2bc6e27457ffccb813e9bd2ffbecbbff",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_0000000000000025ebf2166f97012aac2bc6e27457ffccb813e9bd2ffbecbbff.json",
            ),
            (
                QuorumType::Llmq100_67,
                "0000000000000026e4f1a688a438a9d1f7ac9edac8563b14612fc5d103545008",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_0000000000000026e4f1a688a438a9d1f7ac9edac8563b14612fc5d103545008.json",
            ),
            (
                QuorumType::Llmq100_67,
                "0000000000000029da51c0294b476dc56726212774923ce8e5a80be216a6e4fe",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_0000000000000029da51c0294b476dc56726212774923ce8e5a80be216a6e4fe.json",
            ),
            (
                QuorumType::Llmq100_67,
                "000000000000002d84525b18bf999185fe9ade4602d590b1d736ced080aab49c",
            ) => load_quorum_info_from_file(
                "quorum_info_100_67_000000000000002d84525b18bf999185fe9ade4602d590b1d736ced080aab49c.json",
            ),
            (
                QuorumType::Llmq400_60,
                "000000000000000e73515bfdf034cc1d6501a91f34cbe1c23c1cebcbb4c0a9f3",
            ) => load_quorum_info_from_file(
                "quorum_info_400_60_000000000000000e73515bfdf034cc1d6501a91f34cbe1c23c1cebcbb4c0a9f3.json",
            ),
            (
                QuorumType::Llmq400_60,
                "00000000000000185e2056a59b5c9b593f0e8b7eff113dfdf99bd02be67287fb",
            ) => load_quorum_info_from_file(
                "quorum_info_400_60_00000000000000185e2056a59b5c9b593f0e8b7eff113dfdf99bd02be67287fb.json",
            ),
            (
                QuorumType::Llmq400_60,
                "000000000000002b5d23dae0d422b0c7354e375d0327c12e4b64154b0405939d",
            ) => load_quorum_info_from_file(
                "quorum_info_400_60_000000000000002b5d23dae0d422b0c7354e375d0327c12e4b64154b0405939d.json",
            ),
            (
                QuorumType::Llmq400_60,
                "000000000000002d84525b18bf999185fe9ade4602d590b1d736ced080aab49c",
            ) => load_quorum_info_from_file(
                "quorum_info_400_60_000000000000002d84525b18bf999185fe9ade4602d590b1d736ced080aab49c.json",
            ),



            (_, _) => Err(Error::UnexpectedStructure(format!(
                "no quorum info known for quorum hash {} of type {}",
                hash, quorum_type
            ))),
        }
    }

    fn get_protx_diff_with_masternodes(&self, base_block: Option<u32>, block: u32) -> Result<MasternodeListDiff, Error> {
        match (base_block, block) {
            (None, 2128896) => {
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/production_block_execution_tests/mainnet_genesis_test_data/protx_listdiff_1_2128896.json");
                let file = std::fs::File::open(path.clone()).map_err(|e| {
                    Error::UnexpectedStructure(format!("failed to open protx_diff file: {} at path {}", e, path.to_string_lossy()))
                })?;

                let diff: MasternodeListDiff = serde_json::from_reader(file).map_err(|e| {
                    Error::UnexpectedStructure(format!("failed to deserialize protx_diff: {}", e))
                })?;

                Ok(diff)
            }
            (base, block) => Err(Error::UnexpectedStructure(format!("no pro_tx_diff known from {:?} to {}", base, block)))
        }
    }

    fn verify_instant_lock(&self, _instant_lock: &InstantLock, _max_height: Option<u32>) -> Result<bool, Error> {
        Err(Error::UnexpectedStructure("verify_instant_lock not implemented".to_string()))
    }

    fn verify_chain_lock(&self, _chain_lock: &ChainLock) -> Result<bool, Error> {
        Err(Error::UnexpectedStructure("verify_chain_lock not implemented".to_string()))
    }

    fn masternode_sync_status(&self) -> Result<MnSyncStatus, Error> {
        Err(Error::UnexpectedStructure("masternode_sync_status not implemented".to_string()))
    }

    fn send_raw_transaction(&self, _transaction: &[u8]) -> Result<Txid, Error> {
        Err(Error::UnexpectedStructure("send_raw_transaction not implemented".to_string()))
    }
}