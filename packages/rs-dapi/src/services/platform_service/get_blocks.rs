//! `getBlocks`: up to ten consecutive blocks of the Platform chain.
//!
//! The blocks come from Tenderdash. A client trusts the node it asks unless it sets `prove`.
//! This is for looking blocks up, not for syncing a chain from DAPI, so the cap here works
//! together with a much lower per-IP request limit in the gateway's rate limiter.

use std::ops::RangeInclusive;

use chrono::DateTime;
use dapi_grpc::platform::v0::get_blocks_response::get_blocks_response_v0::{Block, Transaction};
use dapi_grpc::platform::v0::get_blocks_response::{self, GetBlocksResponseV0};
use dapi_grpc::platform::v0::{GetBlocksRequest, GetBlocksResponse, get_blocks_request};
use dapi_grpc::tonic::{Request, Response};
use sha2::{Digest, Sha256};

use super::block_proof::{build_block_proof, check_block_contents};
use super::error_mapping::base64_decode;
use crate::cache::make_cache_key;
use crate::clients::tenderdash_client::{
    Block as TenderdashBlock, BlockTxResult, MAX_TENDERDASH_HEIGHT, ResultBlock,
    ResultBlockResults, ResultCommit,
};
use crate::error::{DAPIResult, DapiError};
use crate::services::platform_service::PlatformServiceImpl;

const MAX_BLOCKS_PER_REQUEST: u32 = 10;

/// Keeps a response within the 4 MiB a gRPC client decodes by default. A block can hold
/// 2 MiB, so ten full blocks would not fit, while any single block does.
const MAX_RESPONSE_BYTES: usize = 4_000_000;

impl PlatformServiceImpl {
    /// Read the requested blocks from Tenderdash, each transaction with its execution result.
    pub(super) async fn get_blocks_impl(
        &self,
        request: Request<GetBlocksRequest>,
    ) -> DAPIResult<Response<GetBlocksResponse>> {
        let request = request.into_inner();
        let (heights, prove) = parse_request(&request)?;

        let key = make_cache_key("get_blocks", &request);
        if let Some(cached) = self.platform_cache.get(&key) {
            return Ok(Response::new(cached));
        }
        let generation = self.platform_cache.generation();

        let blocks = self.fetch_blocks(heights, prove).await?;

        let response = GetBlocksResponse {
            version: Some(get_blocks_response::Version::V0(GetBlocksResponseV0 {
                blocks,
            })),
        };
        self.platform_cache.put_from_generation(
            key,
            &response,
            dapi_grpc::Message::encoded_len(&response),
            generation,
        );

        Ok(Response::new(response))
    }

    /// Read the whole range from Tenderdash in one batched request.
    async fn fetch_blocks(
        &self,
        heights: RangeInclusive<u64>,
        prove: bool,
    ) -> DAPIResult<Vec<Block>> {
        let quorum_type = if prove {
            Some(self.tenderdash_client.validator_quorum_type().await?)
        } else {
            None
        };
        let mut entries = self
            .tenderdash_client
            .block_batch(heights.clone(), prove)
            .await?
            .into_iter();

        collect_blocks(heights, |height| {
            let entry = entries.next().ok_or_else(|| {
                DapiError::Internal(format!("no batch entry for height {height}"))
            })?;
            read_block(
                height,
                entry.block,
                entry.results,
                entry.commit.zip(quorum_type),
            )
        })
    }
}

/// Turn the Tenderdash answers for one height into a block. `results` is only looked at
/// when the block has transactions, and `signed` is `Some` only when asked to prove.
///
/// `None` means the node's chain does not reach this height. A block it holds without its
/// results is an error instead, because that block must not vanish from a range.
fn read_block(
    height: u64,
    block: DAPIResult<ResultBlock>,
    results: DAPIResult<ResultBlockResults>,
    signed: Option<(DAPIResult<ResultCommit>, u32)>,
) -> DAPIResult<Option<Block>> {
    let result = match block {
        Ok(result) => result,
        Err(error) if error.is_missing_tenderdash_height() => return Ok(None),
        Err(error) => return Err(error),
    };
    let Some(block) = result.block else {
        return Ok(None);
    };
    let results = if block.data.txs.is_empty() {
        Vec::new()
    } else {
        results?.txs_results
    };

    let mut built = build_block(height, &result.block_id.hash, block, &results)?;
    if let Some((commit, quorum_type)) = signed {
        let proof = build_block_proof(height, &result.block_id, commit?, quorum_type)?;
        let hashes = built
            .transactions
            .iter()
            .map(|transaction| transaction.hash.clone())
            .collect::<Vec<_>>();
        check_block_contents(&proof, &hashes, &results)?;
        built.proof = Some(proof);
    }

    Ok(Some(built))
}

/// Validate the request, returning the heights it asks for and whether to prove them.
fn parse_request(request: &GetBlocksRequest) -> DAPIResult<(RangeInclusive<u64>, bool)> {
    let Some(get_blocks_request::Version::V0(v0)) = &request.version else {
        return Err(DapiError::InvalidArgument(
            "missing request version".to_string(),
        ));
    };
    if v0.count == 0 || v0.count > MAX_BLOCKS_PER_REQUEST {
        return Err(DapiError::InvalidArgument(format!(
            "count must be between 1 and {MAX_BLOCKS_PER_REQUEST}"
        )));
    }
    if v0.start_height == 0 || v0.start_height > MAX_TENDERDASH_HEIGHT {
        return Err(DapiError::InvalidArgument(format!(
            "start_height must be between 1 and {MAX_TENDERDASH_HEIGHT}"
        )));
    }
    let end_height = v0.start_height + u64::from(v0.count) - 1;
    if end_height > MAX_TENDERDASH_HEIGHT {
        return Err(DapiError::InvalidArgument(format!(
            "the last height must not be above {MAX_TENDERDASH_HEIGHT}"
        )));
    }

    Ok((v0.start_height..=end_height, v0.prove))
}

/// Take the blocks in order, stopping early where the chain ends or the response is full.
/// The first block is always either returned or the error of the query.
fn collect_blocks(
    heights: RangeInclusive<u64>,
    mut read: impl FnMut(u64) -> DAPIResult<Option<Block>>,
) -> DAPIResult<Vec<Block>> {
    let mut blocks = Vec::new();
    let mut response_bytes = 0;
    for height in heights {
        let Some(block) = read(height)? else {
            if blocks.is_empty() {
                return Err(DapiError::NotFound(format!("no block at height {height}")));
            }
            break;
        };
        response_bytes += dapi_grpc::Message::encoded_len(&block);
        if response_bytes > MAX_RESPONSE_BYTES && !blocks.is_empty() {
            break;
        }
        blocks.push(block);
    }
    Ok(blocks)
}

/// Convert a Tenderdash block, pairing each transaction with its execution result.
fn build_block(
    height: u64,
    hash: &str,
    block: TenderdashBlock,
    results: &[BlockTxResult],
) -> DAPIResult<Block> {
    // Results line up with transactions by position, so a list of another length
    // cannot be trusted to give any transaction its own result.
    if results.len() != block.data.txs.len() {
        return Err(DapiError::Internal(format!(
            "block {height} has {} transactions but {} results",
            block.data.txs.len(),
            results.len()
        )));
    }
    let transactions = block
        .data
        .txs
        .iter()
        .zip(results)
        .enumerate()
        .map(|(index, (tx, result))| {
            let state_transition = base64_decode(tx).ok_or_else(|| {
                DapiError::Internal(format!(
                    "transaction {index} of block {height} is not base64"
                ))
            })?;
            let result_data = base64_decode(&result.data).ok_or_else(|| {
                DapiError::Internal(format!("result {index} of block {height} is not base64"))
            })?;
            Ok(Transaction {
                hash: Sha256::digest(&state_transition).to_vec(),
                state_transition,
                code: result.code,
                result_data,
                gas_used: result.gas_used,
            })
        })
        .collect::<DAPIResult<Vec<_>>>()?;

    Ok(Block {
        height,
        hash: decode_hex("hash", hash)?,
        time_ms: parse_time_ms(&block.header.time)?,
        core_chain_locked_height: block.header.core_chain_locked_height,
        app_hash: decode_hex("app_hash", &block.header.app_hash)?,
        proposer_pro_tx_hash: decode_hex(
            "proposer_pro_tx_hash",
            &block.header.proposer_pro_tx_hash,
        )?,
        transactions,
        proof: None,
    })
}

pub(super) fn decode_hex(field: &str, value: &str) -> DAPIResult<Vec<u8>> {
    hex::decode(value)
        .map_err(|error| DapiError::Internal(format!("block {field} is not hex: {error}")))
}

fn parse_time_ms(time: &str) -> DAPIResult<u64> {
    let timestamp = DateTime::parse_from_rfc3339(time)
        .map_err(|error| DapiError::Internal(format!("block time {time:?}: {error}")))?;
    u64::try_from(timestamp.timestamp_millis())
        .map_err(|_| DapiError::Internal(format!("block time {time:?} is before the epoch")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::platform_service::block_proof::tenderdash_commit;
    use dapi_grpc::platform::v0::TenderdashBlockProof;
    use dapi_grpc::platform::v0::get_blocks_request::GetBlocksRequestV0;
    use serde_json::json;

    fn request(start_height: u64, count: u32) -> GetBlocksRequest {
        GetBlocksRequest {
            version: Some(get_blocks_request::Version::V0(GetBlocksRequestV0 {
                start_height,
                count,
                prove: false,
            })),
        }
    }

    fn block_of_size(height: u64, payload_bytes: usize) -> Block {
        Block {
            height,
            transactions: vec![Transaction {
                state_transition: vec![0; payload_bytes],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    /// An error in the shape Tenderdash's RPC answers with.
    fn tenderdash_error(data: &str) -> DapiError {
        DapiError::from_tenderdash_error(json!({
            "code": -32603,
            "message": "Internal error",
            "data": data,
        }))
    }

    fn heights(blocks: &[Block]) -> Vec<u64> {
        blocks.iter().map(|block| block.height).collect()
    }

    #[test]
    fn should_accept_up_to_ten_blocks() {
        assert_eq!(parse_request(&request(5, 1)).unwrap(), (5..=5, false));
        assert_eq!(parse_request(&request(5, 10)).unwrap(), (5..=14, false));
    }

    #[test]
    fn should_reject_a_count_outside_one_to_ten() {
        for count in [0, 11, u32::MAX] {
            assert!(matches!(
                parse_request(&request(5, count)),
                Err(DapiError::InvalidArgument(_))
            ));
        }
    }

    #[test]
    fn should_reject_heights_tenderdash_cannot_hold() {
        for (start_height, count) in [
            (0, 1),
            (MAX_TENDERDASH_HEIGHT + 1, 1),
            (MAX_TENDERDASH_HEIGHT, 2),
            (u64::MAX, 10),
        ] {
            assert!(matches!(
                parse_request(&request(start_height, count)),
                Err(DapiError::InvalidArgument(_))
            ));
        }
        assert_eq!(
            parse_request(&request(MAX_TENDERDASH_HEIGHT, 1)).unwrap(),
            (MAX_TENDERDASH_HEIGHT..=MAX_TENDERDASH_HEIGHT, false)
        );
    }

    #[test]
    fn should_reject_a_request_without_a_version() {
        assert!(matches!(
            parse_request(&GetBlocksRequest { version: None }),
            Err(DapiError::InvalidArgument(_))
        ));
    }

    #[test]
    fn should_return_every_block_of_the_range_in_order() {
        let blocks = collect_blocks(3..=6, |height| Ok(Some(block_of_size(height, 1)))).unwrap();

        assert_eq!(heights(&blocks), [3, 4, 5, 6]);
    }

    #[test]
    fn should_return_the_blocks_that_exist_when_the_range_passes_the_tip() {
        let blocks = collect_blocks(3..=12, |height| {
            Ok((height <= 5).then(|| block_of_size(height, 1)))
        })
        .unwrap();

        assert_eq!(heights(&blocks), [3, 4, 5]);
    }

    #[test]
    fn should_report_not_found_when_the_first_block_is_missing() {
        let result = collect_blocks(3..=12, |_| Ok(None));

        assert!(matches!(result, Err(DapiError::NotFound(_))));
    }

    #[test]
    fn should_fail_rather_than_cut_the_range_short_on_another_error() {
        let result = collect_blocks(3..=12, |height| {
            if height > 5 {
                Err(DapiError::Internal("broken".to_string()))
            } else {
                Ok(Some(block_of_size(height, 1)))
            }
        });

        assert!(matches!(result, Err(DapiError::Internal(_))));
    }

    #[test]
    fn should_fail_rather_than_omit_a_block_whose_results_are_missing() {
        let result = collect_blocks(3..=12, |height| {
            let txs = if height > 5 {
                json!(["AQI="])
            } else {
                json!(null)
            };
            read_block(
                height,
                Ok(tenderdash_block(txs)),
                Err(tenderdash_error("could not find results for height #6")),
                None,
            )
        });

        let error = result.unwrap_err();
        assert!(error.is_missing_tenderdash_height());
    }

    #[test]
    fn should_end_the_range_when_the_block_itself_is_missing() {
        let blocks = collect_blocks(3..=12, |height| {
            let block = if height > 5 {
                Err(tenderdash_error(
                    "height must be less than or equal to the head of the node's blockchain",
                ))
            } else {
                Ok(tenderdash_block(json!(null)))
            };
            read_block(
                height,
                block,
                Err(DapiError::Internal("no results expected".to_string())),
                None,
            )
        })
        .unwrap();

        assert_eq!(heights(&blocks), [3, 4, 5]);
    }

    /// Read one block holding `[1, 2]` with the given commit as its proof.
    fn read_proved_block(commit: ResultCommit) -> DAPIResult<Option<Block>> {
        read_block(
            7,
            Ok(tenderdash_block(json!(["AQI="]))),
            Ok(serde_json::from_value(json!({
                "txs_results": [{ "code": 0, "gas_used": "9" }],
            }))
            .unwrap()),
            Some((Ok(commit), 6)),
        )
    }

    #[test]
    fn should_use_no_results_of_a_block_without_transactions() {
        let block = read_block(
            7,
            Ok(tenderdash_block(json!(null))),
            Err(DapiError::Internal("not looked at".to_string())),
            None,
        )
        .unwrap()
        .unwrap();

        assert!(block.transactions.is_empty());
    }

    #[test]
    fn should_fail_when_the_commit_of_a_proved_block_is_missing() {
        let result = read_block(
            7,
            Ok(tenderdash_block(json!(null))),
            Ok(ResultBlockResults::default()),
            Some((Err(DapiError::Client("no commit".to_string())), 6)),
        );

        assert!(matches!(result, Err(DapiError::Client(_))));
    }

    #[test]
    fn should_not_read_past_the_block_that_fills_the_response() {
        let mut read = Vec::new();

        let blocks = collect_blocks(1..=10, |height| {
            read.push(height);
            Ok(Some(block_of_size(height, 3_000_000)))
        })
        .unwrap();

        assert_eq!(heights(&blocks), [1]);
        assert_eq!(read, [1, 2]);
    }

    fn results_of_the_proved_block() -> [BlockTxResult; 1] {
        [BlockTxResult {
            gas_used: 9,
            ..Default::default()
        }]
    }

    #[test]
    fn should_attach_a_proof_when_asked_to_prove() {
        let commit = tenderdash_commit(
            7,
            "ABCD",
            &[Sha256::digest([1, 2]).to_vec()],
            &results_of_the_proved_block(),
        );

        let block = read_proved_block(commit).unwrap().unwrap();

        let proof = block.proof.unwrap();
        assert_eq!(proof.quorum_type, 6);
        assert_eq!(proof.header.unwrap().height, 7);
        assert_eq!(block.transactions[0].gas_used, 9);
    }

    #[test]
    fn should_fail_rather_than_return_a_proof_the_block_does_not_satisfy() {
        let commit = tenderdash_commit(
            7,
            "ABCD",
            &[Sha256::digest([3, 4]).to_vec()],
            &results_of_the_proved_block(),
        );

        assert!(matches!(
            read_proved_block(commit),
            Err(DapiError::Internal(_))
        ));
    }

    #[test]
    fn should_count_the_proof_in_the_response_size() {
        let blocks = collect_blocks(1..=2, |height| {
            let mut block = block_of_size(height, 2_000_000);
            block.proof = Some(TenderdashBlockProof {
                signature: vec![0; 100_000],
                ..Default::default()
            });
            Ok(Some(block))
        })
        .unwrap();

        assert_eq!(heights(&blocks), [1]);
    }

    #[test]
    fn should_read_the_prove_flag() {
        let mut proved = request(5, 1);
        let Some(get_blocks_request::Version::V0(v0)) = &mut proved.version else {
            unreachable!();
        };
        v0.prove = true;

        assert_eq!(parse_request(&proved).unwrap(), (5..=5, true));
    }

    #[test]
    fn should_stop_before_the_response_passes_its_size_limit() {
        let blocks =
            collect_blocks(1..=10, |height| Ok(Some(block_of_size(height, 1_500_000)))).unwrap();

        assert_eq!(heights(&blocks), [1, 2]);
    }

    #[test]
    fn should_return_a_first_block_larger_than_the_size_limit() {
        let blocks = collect_blocks(1..=10, |height| {
            Ok(Some(block_of_size(height, MAX_RESPONSE_BYTES + 1)))
        })
        .unwrap();

        assert_eq!(heights(&blocks), [1]);
    }

    /// A `block` result in the shape Tenderdash's RPC serves.
    fn tenderdash_block(txs: serde_json::Value) -> ResultBlock {
        serde_json::from_value(json!({
            "block_id": {
                "hash": "ABCD",
                "parts": { "total": 1, "hash": "00" },
                "state_id": "00",
            },
            "block": {
                "header": {
                    "version": { "block": "14", "app": "11" },
                    "chain_id": "dash-1",
                    "height": "7",
                    "core_chain_locked_height": 2300000,
                    "time": "2024-07-29T15:00:00.123456789Z",
                    "app_hash": "0102",
                    "proposer_pro_tx_hash": "0A0B",
                },
                "data": { "txs": txs },
                "core_chain_lock": null,
                "evidence": [],
                "last_commit": null,
            },
        }))
        .unwrap()
    }

    #[test]
    fn should_convert_a_tenderdash_block() {
        let result = tenderdash_block(json!(["AQI=", "AwQ="]));
        let results: ResultBlockResults = serde_json::from_value(json!({
            "height": "7",
            "txs_results": [{ "code": 0 }, { "code": 40100 }],
        }))
        .unwrap();

        let block = build_block(
            7,
            &result.block_id.hash,
            result.block.unwrap(),
            &results.txs_results,
        )
        .unwrap();

        assert!(block.proof.is_none());
        assert_eq!(block.height, 7);
        assert_eq!(block.hash, [0xab, 0xcd]);
        assert_eq!(block.time_ms, 1_722_265_200_123);
        assert_eq!(block.core_chain_locked_height, 2_300_000);
        assert_eq!(block.app_hash, [1, 2]);
        assert_eq!(block.proposer_pro_tx_hash, [10, 11]);
        assert_eq!(block.transactions.len(), 2);
        assert_eq!(block.transactions[0].state_transition, [1, 2]);
        assert_eq!(block.transactions[0].hash, Sha256::digest([1, 2]).to_vec());
        assert_eq!(block.transactions[0].code, 0);
        assert_eq!(block.transactions[1].state_transition, [3, 4]);
        assert_eq!(block.transactions[1].code, 40100);
    }

    #[test]
    fn should_convert_a_block_without_transactions() {
        let result = tenderdash_block(json!(null));

        let block = build_block(7, &result.block_id.hash, result.block.unwrap(), &[]).unwrap();

        assert!(block.transactions.is_empty());
    }

    #[test]
    fn should_fail_when_results_do_not_line_up_with_the_block() {
        let result = tenderdash_block(json!(["AQI=", "AwQ="]));

        assert!(matches!(
            build_block(
                7,
                &result.block_id.hash,
                result.block.unwrap(),
                &[BlockTxResult::default()],
            ),
            Err(DapiError::Internal(_))
        ));
    }

    #[test]
    fn should_fail_on_a_transaction_that_is_not_base64() {
        let result = tenderdash_block(json!(["not base64!"]));

        assert!(matches!(
            build_block(
                7,
                &result.block_id.hash,
                result.block.unwrap(),
                &[BlockTxResult::default()],
            ),
            Err(DapiError::Internal(_))
        ));
    }

    #[test]
    fn should_fail_on_a_block_time_it_cannot_read() {
        assert!(matches!(
            parse_time_ms("yesterday"),
            Err(DapiError::Internal(_))
        ));
        assert!(matches!(
            parse_time_ms("1969-12-31T23:59:59Z"),
            Err(DapiError::Internal(_))
        ));
    }
}
